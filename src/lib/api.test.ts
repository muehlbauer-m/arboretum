import { invoke } from "@tauri-apps/api/core";
import { getConfig, saveConfig, generateNewsletter, listNewsletters, readNewsletter } from "./api";

const mockInvoke = vi.mocked(invoke);

beforeEach(() => {
  vi.clearAllMocks();
});

describe("api — getConfig", () => {
  it("calls invoke with get_config", async () => {
    mockInvoke.mockResolvedValueOnce({ gemini_api_key: "key123" });
    const result = await getConfig();
    expect(mockInvoke).toHaveBeenCalledWith("get_config");
    expect(result).toEqual({ gemini_api_key: "key123" });
  });
});

describe("api — saveConfig", () => {
  it("calls invoke with save_config and config payload", async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    const config = { gemini_api_key: "abc" } as any;
    await saveConfig(config);
    expect(mockInvoke).toHaveBeenCalledWith("save_config", { config });
  });
});

describe("api — generateNewsletter", () => {
  it("calls invoke with correct snake_case params", async () => {
    mockInvoke.mockResolvedValueOnce([]);
    await generateNewsletter(
      [{ query: "machine learning" }],
      ["openalex", "arxiv"],
      50,
      90
    );
    expect(mockInvoke).toHaveBeenCalledWith("generate_newsletter", {
      topics: [{ query: "machine learning" }],
      sources: ["openalex", "arxiv"],
      maxPapers: 50,
      daysBack: 90,
    });
  });

  it("returns results array", async () => {
    const expected = [{ topic: "ml", title: "ML Digest", path: "/foo.md", error: null }];
    mockInvoke.mockResolvedValueOnce(expected);
    const result = await generateNewsletter([{ query: "ml" }], ["arxiv"], 20, 30);
    expect(result).toEqual(expected);
  });
});

describe("api — listNewsletters", () => {
  it("calls invoke with output_dir", async () => {
    mockInvoke.mockResolvedValueOnce([]);
    await listNewsletters("/some/path");
    expect(mockInvoke).toHaveBeenCalledWith("list_newsletters", { outputDir: "/some/path" });
  });
});

describe("api — readNewsletter", () => {
  it("calls invoke with path", async () => {
    mockInvoke.mockResolvedValueOnce("# Newsletter content");
    const result = await readNewsletter("/path/to/file.md");
    expect(mockInvoke).toHaveBeenCalledWith("read_newsletter", { path: "/path/to/file.md" });
    expect(result).toBe("# Newsletter content");
  });
});
