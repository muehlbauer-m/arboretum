"""
Quality comparison: cloud baseline (Claude API or Gemini) vs local qwen3:8b via Ollama.

Mirrors the prompts in src-tauri/src/gemini.rs so the comparison is apples-to-apples.

Run (Claude as baseline):
    ANTHROPIC_API_KEY=... uv run --with httpx scripts/quality-test/harness.py

To use Gemini instead:
    BASELINE=gemini GEMINI_API_KEY=... uv run --with httpx scripts/quality-test/harness.py
"""

import asyncio
import json
import os
import sys
import time
from pathlib import Path

import httpx

# ─── Config ──────────────────────────────────────────────────────────────────

QUERY = "small area estimation fay-herriot models"
TITLE_HINT = "Small Area Estimation"
DAYS_BACK = 730  # Fay-Herriot is a slower-moving field; widen the window
MAX_PAPERS = 40
RESEARCH_DESCRIPTION = (
    "I'm developing a hierarchical Bayesian Fay-Herriot model with spatial random "
    "effects for small area estimation of poverty rates, using INLA for inference."
)
KEY_TERMS = [
    "fay-herriot",
    "small area estimation",
    "spatial random effects",
    "hierarchical bayesian",
    "INLA",
    "poverty mapping",
]

BASELINE = os.environ.get("BASELINE", "claude").lower()  # "claude" or "gemini"

GEMINI_MODEL = "gemini-2.5-flash"
GEMINI_URL = (
    f"https://generativelanguage.googleapis.com/v1beta/models/{GEMINI_MODEL}:generateContent"
)
ANTHROPIC_URL = "https://api.anthropic.com/v1/messages"
ANTHROPIC_MODEL = os.environ.get("ANTHROPIC_MODEL", "claude-sonnet-4-6")
ANTHROPIC_VERSION = "2023-06-01"
OLLAMA_URL = "http://127.0.0.1:11434/api/generate"
OLLAMA_MODEL = os.environ.get("OLLAMA_MODEL", "qwen3:8b")

OUT_DIR = Path(__file__).parent / "output"

# ─── OpenAlex fetch ──────────────────────────────────────────────────────────


def decode_inverted_index(inv: dict | None) -> str:
    if not inv:
        return ""
    pos_word: dict[int, str] = {}
    for word, positions in inv.items():
        for p in positions:
            pos_word[p] = word
    return " ".join(pos_word[k] for k in sorted(pos_word.keys()))


async def fetch_papers(client: httpx.AsyncClient, query: str, n: int, days: int) -> list[dict]:
    from datetime import datetime, timedelta

    from_date = (datetime.utcnow() - timedelta(days=days)).strftime("%Y-%m-%d")
    params = {
        "search": query,
        "filter": f"from_publication_date:{from_date},type:article",
        "sort": "relevance_score:desc",
        "per-page": min(n, 50),
        "select": "id,title,abstract_inverted_index,authorships,publication_date,doi,primary_location",
        "mailto": "research.newsletter@local",
    }
    r = await client.get("https://api.openalex.org/works", params=params, timeout=30)
    r.raise_for_status()
    data = r.json()
    out = []
    for w in data.get("results", []):
        title = (w.get("title") or "").strip()
        if not title:
            continue
        abstract = decode_inverted_index(w.get("abstract_inverted_index"))
        authors = ", ".join(
            a["author"]["display_name"]
            for a in (w.get("authorships") or [])[:3]
            if a.get("author", {}).get("display_name")
        )
        loc = w.get("primary_location") or {}
        url = loc.get("landing_page_url") or w.get("doi") or w.get("id") or ""
        out.append(
            {
                "title": title,
                "abstract_text": abstract,
                "authors": authors,
                "date": w.get("publication_date") or "",
                "url": url,
                "source": "OpenAlex",
            }
        )
    return out


# ─── Prompt builders (mirror gemini.rs) ──────────────────────────────────────


def build_curation_prompt(papers: list[dict], user_query: str) -> str:
    paper_list_parts = []
    for i, p in enumerate(papers[:60]):
        ab = p["abstract_text"]
        if len(ab) > 600:
            ab = ab[:600]
        paper_list_parts.append(
            f"Paper {i + 1}: {p['title']} by {p['authors']}.\n"
            f"Abstract: {ab}\n"
            f"URL: {p['url']}  Source: {p['source']}"
        )
    paper_list = "\n---\n".join(paper_list_parts)
    n = min(len(papers), 60)
    return f'''You are a research newsletter curator. The reader is interested in:
"{user_query}"

Below are {n} papers from academic databases.

Select the 10 most relevant and impactful papers. For each write:
- A clear, jargon-free 2-3 sentence summary (what it does, why it matters)
- A relevance tag (e.g. Machine Learning, Biology, Climate, Economics…)

End with a 2-sentence overview of the theme across these papers.

Format as Markdown: ## for titles linked to paper URL, paragraphs for summaries, **bold** for tags. Do NOT wrap in code fences.

{paper_list}'''


def build_conflict_prompt(papers: list[dict], desc: str, terms: list[str]) -> str:
    terms_str = ", ".join(terms)
    parts = []
    for i, p in enumerate(papers):
        ab = p["abstract_text"]
        if len(ab) > 1200:
            ab = ab[:1200]
        parts.append(
            f"[{i + 1}] Title: {p['title']}\n"
            f"Authors: {p['authors']}\n"
            f"Date: {p['date']}\n"
            f"URL: {p['url']}\n"
            f"Abstract: {ab}"
        )
    paper_list = "\n---\n".join(parts)
    n = len(papers)
    return f'''You are an expert academic conflict/overlap detector. A researcher needs to know which recent papers overlap with their specific work.

RESEARCHER'S WORK:
"{desc}"

KEY TERMS: [{terms_str}]

Below are {n} papers to evaluate. Score each paper 0-100 on how much it overlaps with or competes with the researcher's work:
- 0-24: No meaningful overlap
- 25-49: Some topical overlap but different focus (low threat)
- 50-74: Significant methodological or topical overlap (moderate threat)
- 75-89: High overlap, potentially competing work (significant threat)
- 90-100: Very high overlap, nearly identical research direction (critical threat)

IMPORTANT: Err on the side of INCLUDING papers rather than excluding. A false positive is far less costly than a false negative. If in doubt, score higher.

For EVERY paper scoring 25 or above, return a JSON object. Return a JSON array of these objects.

Each object must have:
- "paper_index": the paper number (1-based integer)
- "score": overlap score (integer 0-100)
- "title": the paper's title
- "url": the paper's URL
- "authors": the paper's authors
- "date": the paper's date
- "summary": 1-2 sentence summary of the paper
- "overlap": explanation of how it overlaps with the researcher's work
- "difference": what makes it different from the researcher's work
- "threat_level": one of "low", "moderate", "significant", "critical"
- "matched_terms": array of key terms from the researcher's list that match

If no papers score 25 or above, return an empty JSON array: [].

Respond with ONLY the JSON array, no markdown fences.

PAPERS:
{paper_list}'''


# ─── Provider clients ────────────────────────────────────────────────────────


async def call_claude(
    client: httpx.AsyncClient, api_key: str, prompt: str, *, temperature: float, max_tokens: int
) -> tuple[str, dict]:
    """POST to the Anthropic Messages API. Mirrors src-tauri/src/claude_api.rs."""
    body = {
        "model": ANTHROPIC_MODEL,
        "max_tokens": max_tokens,
        "temperature": temperature,
        "messages": [{"role": "user", "content": prompt}],
    }
    t0 = time.time()
    r = await client.post(
        ANTHROPIC_URL,
        json=body,
        headers={
            "x-api-key": api_key,
            "anthropic-version": ANTHROPIC_VERSION,
            "content-type": "application/json",
        },
        timeout=300,
    )
    dt = time.time() - t0
    r.raise_for_status()
    data = r.json()
    parts = [p.get("text", "") for p in data.get("content", []) if p.get("type") == "text"]
    return "".join(parts).strip(), {"seconds": round(dt, 2), "usage": data.get("usage", {})}


async def call_gemini(
    client: httpx.AsyncClient, api_key: str, prompt: str, *, temperature: float, max_tokens: int
) -> tuple[str, dict]:
    body = {
        "contents": [{"parts": [{"text": prompt}]}],
        "generationConfig": {
            "temperature": temperature,
            "maxOutputTokens": max_tokens,
            "thinkingConfig": {"thinkingBudget": 0},
        },
    }
    t0 = time.time()
    r = await client.post(
        f"{GEMINI_URL}?key={api_key}", json=body, timeout=300
    )
    dt = time.time() - t0
    r.raise_for_status()
    data = r.json()
    parts = data["candidates"][0]["content"].get("parts", [])
    text = "".join(p.get("text", "") for p in parts)
    usage = data.get("usageMetadata", {})
    return text, {"seconds": round(dt, 2), "usage": usage}


async def call_ollama(
    client: httpx.AsyncClient, model: str, prompt: str, *, temperature: float, num_predict: int
) -> tuple[str, dict]:
    body = {
        "model": model,
        "prompt": prompt,
        "stream": False,
        "options": {
            "temperature": temperature,
            "num_predict": num_predict,
            "num_ctx": 32768,
        },
    }
    t0 = time.time()
    r = await client.post(OLLAMA_URL, json=body, timeout=3600)
    dt = time.time() - t0
    r.raise_for_status()
    data = r.json()
    return data["response"], {
        "seconds": round(dt, 2),
        "eval_count": data.get("eval_count"),
        "eval_duration_ns": data.get("eval_duration"),
        "tokens_per_sec": (
            round(data["eval_count"] / (data["eval_duration"] / 1e9), 2)
            if data.get("eval_count") and data.get("eval_duration")
            else None
        ),
    }


# ─── Main ────────────────────────────────────────────────────────────────────


async def main() -> int:
    if BASELINE not in {"claude", "gemini"}:
        print(f"ERROR: BASELINE must be 'claude' or 'gemini', got {BASELINE!r}", file=sys.stderr)
        return 2

    api_key = None
    if BASELINE == "gemini":
        api_key = os.environ.get("GEMINI_API_KEY")
        if not api_key:
            print("ERROR: BASELINE=gemini requires GEMINI_API_KEY env var.", file=sys.stderr)
            return 2
    else:
        api_key = os.environ.get("ANTHROPIC_API_KEY")
        if not api_key:
            print("ERROR: BASELINE=claude requires ANTHROPIC_API_KEY env var.", file=sys.stderr)
            return 2

    OUT_DIR.mkdir(parents=True, exist_ok=True)

    async def run_baseline(prompt: str, *, temperature: float, max_tokens: int):
        if BASELINE == "claude":
            return await call_claude(
                client, api_key, prompt, temperature=temperature, max_tokens=max_tokens
            )
        return await call_gemini(
            client, api_key, prompt, temperature=temperature, max_tokens=max_tokens
        )

    async with httpx.AsyncClient() as client:
        print(f"[1/4] Fetching papers from OpenAlex for: {QUERY!r}")
        papers = await fetch_papers(client, QUERY, MAX_PAPERS, DAYS_BACK)
        print(f"     -> {len(papers)} papers")
        if len(papers) < 5:
            print("ERROR: too few papers; widen days_back or change query.", file=sys.stderr)
            return 3
        (OUT_DIR / "papers.json").write_text(json.dumps(papers, indent=2), encoding="utf-8")

        baseline_label = BASELINE
        baseline_suffix = BASELINE

        # Curation
        cur_prompt = build_curation_prompt(papers, QUERY)
        print(f"[2/4] Curation prompt: {len(cur_prompt):,} chars")

        print(f"     -> {baseline_label} …")
        base_cur, base_cur_meta = await run_baseline(cur_prompt, temperature=0.7, max_tokens=8192)
        print(f"        {base_cur_meta['seconds']}s")
        (OUT_DIR / f"curation_{baseline_suffix}.md").write_text(base_cur, encoding="utf-8")

        print(f"     -> Ollama ({OLLAMA_MODEL}) …")
        oll_cur, oll_cur_meta = await call_ollama(
            client, OLLAMA_MODEL, cur_prompt, temperature=0.7, num_predict=8192
        )
        print(f"        {oll_cur_meta['seconds']}s, "
              f"{oll_cur_meta.get('tokens_per_sec')} tok/s "
              f"({oll_cur_meta.get('eval_count')} tok)")
        (OUT_DIR / "curation_ollama.md").write_text(oll_cur, encoding="utf-8")

        # Conflict scan
        conf_prompt = build_conflict_prompt(papers, RESEARCH_DESCRIPTION, KEY_TERMS)
        print(f"[3/4] Conflict-scan prompt: {len(conf_prompt):,} chars")

        print(f"     -> {baseline_label} …")
        base_conf, base_conf_meta = await run_baseline(conf_prompt, temperature=0.3, max_tokens=16384)
        print(f"        {base_conf_meta['seconds']}s")
        (OUT_DIR / f"conflict_{baseline_suffix}.json.txt").write_text(base_conf, encoding="utf-8")

        print(f"     -> Ollama ({OLLAMA_MODEL}) …")
        oll_conf, oll_conf_meta = await call_ollama(
            client, OLLAMA_MODEL, conf_prompt, temperature=0.3, num_predict=16384
        )
        print(f"        {oll_conf_meta['seconds']}s, "
              f"{oll_conf_meta.get('tokens_per_sec')} tok/s "
              f"({oll_conf_meta.get('eval_count')} tok)")
        (OUT_DIR / "conflict_ollama.json.txt").write_text(oll_conf, encoding="utf-8")

        # Validate JSON adherence
        def parse_jsonish(s: str):
            s = s.strip()
            for fence in ("```json", "```"):
                if s.startswith(fence):
                    s = s[len(fence):].lstrip()
            if s.endswith("```"):
                s = s[: -3].rstrip()
            try:
                return json.loads(s), None
            except json.JSONDecodeError as e:
                lb = s.find("[")
                rb = s.rfind("]")
                if lb >= 0 and rb > lb:
                    try:
                        return json.loads(s[lb : rb + 1]), f"recovered (extra prose around array): {e}"
                    except json.JSONDecodeError as e2:
                        return None, str(e2)
                return None, str(e)

        base_json, base_err = parse_jsonish(base_conf)
        oll_json, oll_err = parse_jsonish(oll_conf)

        # Summary report
        print("[4/4] Writing summary …")
        summary = {
            "query": QUERY,
            "papers": len(papers),
            "baseline": baseline_label,
            "curation": {
                baseline_suffix: {
                    "chars": len(base_cur),
                    "seconds": base_cur_meta["seconds"],
                    **({"usage": base_cur_meta["usage"]} if "usage" in base_cur_meta else {}),
                },
                "ollama": {
                    "chars": len(oll_cur),
                    "seconds": oll_cur_meta["seconds"],
                    "tokens_per_sec": oll_cur_meta.get("tokens_per_sec"),
                    "eval_count": oll_cur_meta.get("eval_count"),
                },
            },
            "conflict": {
                baseline_suffix: {
                    "chars": len(base_conf),
                    "seconds": base_conf_meta["seconds"],
                    **({"usage": base_conf_meta["usage"]} if "usage" in base_conf_meta else {}),
                    "valid_json": base_err is None,
                    "json_error": base_err,
                    "flagged_papers": len(base_json) if isinstance(base_json, list) else None,
                },
                "ollama": {
                    "chars": len(oll_conf),
                    "seconds": oll_conf_meta["seconds"],
                    "tokens_per_sec": oll_conf_meta.get("tokens_per_sec"),
                    "eval_count": oll_conf_meta.get("eval_count"),
                    "valid_json": oll_err is None,
                    "json_error": oll_err,
                    "flagged_papers": len(oll_json) if isinstance(oll_json, list) else None,
                },
            },
        }
        (OUT_DIR / "summary.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")
        print(json.dumps(summary, indent=2))

    return 0


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
