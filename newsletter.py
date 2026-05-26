"""
Research Newsletter Generator
UI-driven, multi-source research discovery with AI-powered keyword extraction.

Dependencies: certifi
"""
# /// script
# requires-python = ">=3.12"
# dependencies = ["certifi"]
# ///

import tkinter as tk
from tkinter import ttk, scrolledtext, messagebox
import threading
import urllib.request
import urllib.parse
import json
import re
import os
import ssl
import certifi
from datetime import datetime, timedelta

SSL_CTX = ssl.create_default_context(cafile=certifi.where())
GEMINI_API_KEY = os.environ.get("GEMINI_API_KEY", "")
OUTPUT_DIR = os.path.dirname(os.path.abspath(__file__))


# ---------------------------------------------------------------------------
# Pipeline
# ---------------------------------------------------------------------------

def extract_keywords(natural_query: str) -> dict:
    """Use Gemini to convert a natural-language research interest into search keywords."""
    prompt = (
        f'Convert this research interest into optimal academic search keywords.\n\n'
        f'User interest: "{natural_query}"\n\n'
        f'Return ONLY a JSON object with two keys:\n'
        f'  "keywords": a short search string (3-8 words, no special chars)\n'
        f'  "title": a concise newsletter title (3-5 words)\n\n'
        f'Example: {{"keywords": "large language models drug discovery", "title": "LLMs in Drug Discovery"}}\n\n'
        f'Respond with valid JSON only, no markdown fences.'
    )

    data = _gemini_call(prompt, temperature=0.2, max_tokens=256)
    text = data["candidates"][0]["content"]["parts"][0]["text"].strip()
    return _parse_json_response(text, natural_query)


def search_openalex(keywords: str, max_results: int = 50, days_back: int = 90) -> list[dict]:
    """Search OpenAlex — a free, open database of 250M+ academic works across all fields."""
    from_date = (datetime.now() - timedelta(days=days_back)).strftime("%Y-%m-%d")
    query = urllib.parse.quote(keywords)
    url = (
        f"https://api.openalex.org/works"
        f"?search={query}"
        f"&filter=from_publication_date:{from_date},type:article"
        f"&sort=relevance_score:desc"
        f"&per-page={min(max_results, 50)}"
        f"&select=id,title,abstract_inverted_index,authorships,publication_date,doi,primary_location"
        f"&mailto=research.newsletter@local"
    )

    with urllib.request.urlopen(url, context=SSL_CTX) as resp:
        raw = json.loads(resp.read().decode("utf-8"))

    papers = []
    for work in raw.get("results", []):
        abstract = _decode_inverted_index(work.get("abstract_inverted_index"))
        authors = [
            a["author"]["display_name"]
            for a in work.get("authorships", [])[:3]
            if a.get("author")
        ]
        loc = work.get("primary_location") or {}
        link = loc.get("landing_page_url") or work.get("doi") or work.get("id", "")

        papers.append({
            "title": (work.get("title") or "").strip(),
            "abstract": abstract,
            "authors": ", ".join(authors),
            "date": work.get("publication_date", ""),
            "url": link,
            "source": "OpenAlex",
        })

    return [p for p in papers if p["title"]]


def search_arxiv(keywords: str, max_results: int = 50) -> list[dict]:
    """Search arXiv preprint server."""
    query = urllib.parse.quote(f"all:{keywords}")
    url = (
        f"http://export.arxiv.org/api/query"
        f"?search_query={query}"
        f"&start=0&max_results={max_results}"
        f"&sortBy=submittedDate&sortOrder=descending"
    )

    with urllib.request.urlopen(url, context=SSL_CTX) as resp:
        xml = resp.read().decode("utf-8")

    papers = []
    for entry_match in re.finditer(r"<entry>(.*?)</entry>", xml, re.DOTALL):
        entry = entry_match.group(1)

        def extract(tag, e=entry):
            m = re.search(rf"<{tag}>(.*?)</{tag}>", e, re.DOTALL)
            return m.group(1).strip() if m else ""

        authors = re.findall(r"<author>\s*<name>(.*?)</name>", entry)[:3]
        papers.append({
            "title": re.sub(r"\s+", " ", extract("title")),
            "abstract": re.sub(r"\s+", " ", extract("summary")),
            "authors": ", ".join(authors),
            "date": extract("published"),
            "url": extract("id"),
            "source": "arXiv",
        })

    return [p for p in papers if p["title"]]


def summarize_with_gemini(papers: list[dict], user_query: str, section_title: str) -> str:
    """Curate and summarize papers with Gemini."""
    paper_list = "\n---\n".join(
        f"Paper {i + 1}: {p['title']} by {p['authors']}.\n"
        f"Abstract: {p['abstract'][:600]}\n"
        f"URL: {p['url']}  Source: {p['source']}"
        for i, p in enumerate(papers[:60])
    )

    prompt = (
        f'You are a research newsletter curator. The reader is interested in:\n'
        f'"{user_query}"\n\n'
        f'Below are {min(len(papers), 60)} papers from academic databases.\n\n'
        f'Select the 10 most relevant and impactful papers. For each write:\n'
        f'- A clear, jargon-free 2-3 sentence summary (what it does, why it matters)\n'
        f'- A relevance tag (e.g. Machine Learning, Biology, Climate, Economics…)\n\n'
        f'End with a 2-sentence overview of the theme across these papers.\n\n'
        f'Format as Markdown: ## for titles linked to paper URL, paragraphs for summaries, '
        f'**bold** for tags. Do NOT wrap in code fences.\n\n'
        f'{paper_list}'
    )

    data = _gemini_call(prompt, temperature=0.7, max_tokens=65536, thinking=False)

    candidate = data["candidates"][0]
    finish_reason = candidate.get("finishReason", "UNKNOWN")

    # Concatenate all text parts (Gemini may split long responses)
    parts = candidate.get("content", {}).get("parts", [])
    text = "".join(p.get("text", "") for p in parts if p.get("text"))

    if finish_reason not in ("STOP", "MAX_TOKENS"):
        # Unexpected stop — surface it in the output
        text += f"\n\n*(Response ended early — finishReason: {finish_reason})*"
    elif finish_reason == "MAX_TOKENS":
        text += "\n\n*(Note: response was capped at the token limit — try reducing Max papers)*"

    text = re.sub(r"^```(?:html|markdown|md)?\n?", "", text)
    text = re.sub(r"\n?```$", "", text)
    return text


def save_newsletter(content: str, title: str) -> str:
    today = datetime.now().strftime("%Y-%m-%d")
    today_readable = datetime.now().strftime("%A, %B %d, %Y")
    markdown = (
        f"# {title}\n"
        f"*{today_readable}*\n\n"
        f"---\n\n"
        f"{content}\n\n"
        f"---\n\n"
        f"*Generated automatically via OpenAlex & arXiv using Gemini.*\n"
    )
    filename = os.path.join(OUTPUT_DIR, f"newsletter-{today}.md")
    with open(filename, "w", encoding="utf-8") as f:
        f.write(markdown)
    return filename


def run_pipeline(query: str, sources: list[str], max_papers: int, days_back: int,
                 status_cb) -> tuple[str | None, str | None]:
    """Full pipeline: natural language → keywords → fetch → summarize → save."""
    status_cb("Asking Gemini to extract search keywords…")
    kw_data = extract_keywords(query)
    keywords = kw_data["keywords"]
    title = kw_data.get("title", "Research Digest")
    status_cb(f'Keywords: "{keywords}"  |  Title: "{title}"')

    all_papers: list[dict] = []
    per_source = max(10, max_papers // len(sources)) if sources else max_papers

    if "openalex" in sources:
        status_cb(f"Searching OpenAlex (last {days_back} days)…")
        papers = search_openalex(keywords, max_results=per_source, days_back=days_back)
        all_papers.extend(papers)
        status_cb(f"  → {len(papers)} papers from OpenAlex")

    if "arxiv" in sources:
        status_cb("Searching arXiv…")
        papers = search_arxiv(keywords, max_results=per_source)
        all_papers.extend(papers)
        status_cb(f"  → {len(papers)} papers from arXiv")

    if not all_papers:
        return None, "No papers found. Try broader or different keywords."

    status_cb(f"Sending {len(all_papers)} papers to Gemini for curation…")
    summary = summarize_with_gemini(all_papers, query, title)

    status_cb("Saving newsletter…")
    path = save_newsletter(summary, title)
    return path, None


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _parse_json_response(text: str, fallback_query: str) -> dict:
    """Extract a JSON object from a Gemini response, with a safe fallback."""
    # Strip markdown fences (```json … ``` or ``` … ```)
    text = re.sub(r"```[a-z]*\n?", "", text).strip().strip("`").strip()

    # Try the cleaned text first
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        pass

    # Try to extract the first {...} block from the text
    m = re.search(r"\{[^{}]+\}", text, re.DOTALL)
    if m:
        try:
            return json.loads(m.group(0))
        except json.JSONDecodeError:
            pass

    # Last resort: derive keywords directly from the query
    words = re.sub(r"[^\w\s]", "", fallback_query).lower().split()
    keywords = " ".join(words[:8])
    title = " ".join(w.capitalize() for w in words[:4])
    return {"keywords": keywords, "title": title}


def _gemini_call(prompt: str, temperature: float = 0.7, max_tokens: int = 8192,
                 thinking: bool = False) -> dict:
    generation_config: dict = {
        "temperature": temperature,
        "maxOutputTokens": max_tokens,
    }
    if not thinking:
        # Disable thinking to preserve all output tokens for actual content
        generation_config["thinkingConfig"] = {"thinkingBudget": 0}

    body = json.dumps({
        "contents": [{"parts": [{"text": prompt}]}],
        "generationConfig": generation_config,
    }).encode("utf-8")
    url = (
        f"https://generativelanguage.googleapis.com/v1beta/"
        f"models/gemini-2.5-flash:generateContent?key={GEMINI_API_KEY}"
    )
    req = urllib.request.Request(url, data=body, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, context=SSL_CTX, timeout=120) as resp:
        return json.loads(resp.read().decode("utf-8"))


def _decode_inverted_index(inv_idx: dict | None) -> str:
    if not inv_idx:
        return ""
    words: dict[int, str] = {}
    for word, positions in inv_idx.items():
        for pos in positions:
            words[pos] = word
    return " ".join(words[p] for p in sorted(words))


# ---------------------------------------------------------------------------
# GUI
# ---------------------------------------------------------------------------

PLACEHOLDER = (
    "Describe what you're curious about — e.g.\n"
    "  \"How are transformer models being applied to protein folding?\"\n"
    "  \"Recent advances in solid-state batteries\"\n"
    "  \"Economics of carbon markets and climate policy\""
)


class App(tk.Tk):
    def __init__(self):
        super().__init__()
        self.title("Research Newsletter Generator")
        self.geometry("720x600")
        self.minsize(600, 500)
        self.configure(bg="#f5f5f5")
        self._build_ui()

    # ------------------------------------------------------------------
    def _build_ui(self):
        pad = dict(padx=14, pady=5)

        # Header
        hdr = tk.Frame(self, bg="#1a1a2e", pady=10)
        hdr.pack(fill="x")
        tk.Label(hdr, text="Research Newsletter Generator",
                 font=("Segoe UI", 14, "bold"), bg="#1a1a2e", fg="white").pack()
        tk.Label(hdr, text="Powered by OpenAlex · arXiv · Gemini",
                 font=("Segoe UI", 9), bg="#1a1a2e", fg="#aaaacc").pack()

        # Topic input
        tk.Label(self, text="What are you interested in?",
                 font=("Segoe UI", 10, "bold"), bg="#f5f5f5").pack(anchor="w", **pad)

        self.topic = scrolledtext.ScrolledText(
            self, height=5, font=("Segoe UI", 10), wrap=tk.WORD,
            relief="solid", borderwidth=1, fg="gray"
        )
        self.topic.pack(fill="x", padx=14)
        self.topic.insert("1.0", PLACEHOLDER)
        self.topic.bind("<FocusIn>", self._on_focus_in)
        self.topic.bind("<FocusOut>", self._on_focus_out)

        # Options row
        opts = tk.Frame(self, bg="#f5f5f5")
        opts.pack(fill="x", **pad)

        # Sources
        tk.Label(opts, text="Sources:", font=("Segoe UI", 9, "bold"),
                 bg="#f5f5f5").grid(row=0, column=0, sticky="w")
        self._openalex = tk.BooleanVar(value=True)
        self._arxiv = tk.BooleanVar(value=True)
        ttk.Checkbutton(opts, text="OpenAlex (all academic fields)",
                        variable=self._openalex).grid(row=0, column=1, sticky="w", padx=6)
        ttk.Checkbutton(opts, text="arXiv (preprints)",
                        variable=self._arxiv).grid(row=0, column=2, sticky="w", padx=6)

        # Max papers
        tk.Label(opts, text="Max papers:", font=("Segoe UI", 9, "bold"),
                 bg="#f5f5f5").grid(row=1, column=0, sticky="w", pady=4)
        self._max_papers = tk.IntVar(value=50)
        ttk.Spinbox(opts, from_=10, to=100, increment=10,
                    textvariable=self._max_papers, width=5).grid(row=1, column=1, sticky="w", padx=6)

        # Days back
        tk.Label(opts, text="Date range:", font=("Segoe UI", 9, "bold"),
                 bg="#f5f5f5").grid(row=1, column=2, sticky="w", padx=(12, 0))
        self._days = tk.IntVar(value=90)
        days_cb = ttk.Combobox(opts, textvariable=self._days, width=14,
                               values=[7, 14, 30, 60, 90, 180, 365], state="readonly")
        days_cb.grid(row=1, column=3, sticky="w", padx=6)
        tk.Label(opts, text="days back", bg="#f5f5f5").grid(row=1, column=4, sticky="w")

        # Generate button
        self._btn = ttk.Button(self, text="⚡  Generate Newsletter",
                               command=self._start, style="Accent.TButton")
        self._btn.pack(pady=8)

        # Status
        self._status = tk.StringVar(value="Ready.")
        tk.Label(self, textvariable=self._status, font=("Segoe UI", 9),
                 fg="#555555", bg="#f5f5f5").pack()

        # Progress
        self._progress = ttk.Progressbar(self, mode="indeterminate", length=400)
        self._progress.pack(pady=2)

        # Log
        tk.Label(self, text="Log", font=("Segoe UI", 9, "bold"),
                 bg="#f5f5f5").pack(anchor="w", padx=14, pady=(6, 0))
        self._log = scrolledtext.ScrolledText(
            self, height=10, font=("Consolas", 9), state="disabled",
            relief="solid", borderwidth=1
        )
        self._log.pack(fill="both", expand=True, padx=14, pady=(0, 14))

    # ------------------------------------------------------------------
    def _on_focus_in(self, _):
        if self.topic.get("1.0", "end-1c").strip() == PLACEHOLDER.strip():
            self.topic.delete("1.0", "end")
            self.topic.configure(fg="black")

    def _on_focus_out(self, _):
        if not self.topic.get("1.0", "end-1c").strip():
            self.topic.configure(fg="gray")
            self.topic.insert("1.0", PLACEHOLDER)

    def _log_line(self, msg: str):
        self._log.configure(state="normal")
        ts = datetime.now().strftime("%H:%M:%S")
        self._log.insert("end", f"[{ts}]  {msg}\n")
        self._log.see("end")
        self._log.configure(state="disabled")
        self._status.set(msg[:90])

    def _start(self):
        query = self.topic.get("1.0", "end-1c").strip()
        if not query or query == PLACEHOLDER.strip():
            messagebox.showwarning("Input needed", "Please describe your research interests.")
            return

        sources = []
        if self._openalex.get():
            sources.append("openalex")
        if self._arxiv.get():
            sources.append("arxiv")
        if not sources:
            messagebox.showwarning("No source selected", "Enable at least one source.")
            return

        self._btn.configure(state="disabled")
        self._progress.start(12)
        self._log_line(f'Query: "{query[:80]}"')

        def worker():
            try:
                path, err = run_pipeline(
                    query, sources,
                    self._max_papers.get(),
                    self._days.get(),
                    lambda m: self.after(0, self._log_line, m),
                )
                self.after(0, self._done, path, err)
            except Exception as exc:
                self.after(0, self._error, str(exc))

        threading.Thread(target=worker, daemon=True).start()

    def _done(self, path: str | None, err: str | None):
        self._progress.stop()
        self._btn.configure(state="normal")
        if err:
            self._log_line(f"ERROR: {err}")
            messagebox.showerror("Error", err)
        else:
            self._log_line(f"Saved: {path}")
            messagebox.showinfo("Done!", f"Newsletter saved to:\n{path}")

    def _error(self, msg: str):
        self._progress.stop()
        self._btn.configure(state="normal")
        self._log_line(f"ERROR: {msg}")
        messagebox.showerror("Unexpected error", msg)


# ---------------------------------------------------------------------------

if __name__ == "__main__":
    app = App()
    app.mainloop()
