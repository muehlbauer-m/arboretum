import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import Home from "./Home";
import { GenerationProvider } from "../lib/GenerationContext";
import { ConflictProvider } from "../lib/ConflictContext";

const mockInvoke = vi.mocked(invoke);
const mockListen = vi.mocked(listen);

const mockConfig = {
  gemini_api_key: "test-key",
  claude_api_key: "",
  ai_provider: "gemini",
  output_dir: "/tmp/newsletters",
  default_sources: ["openalex", "arxiv"],
  default_max_papers: 50,
  default_days_back: 90,
  email: { enabled: false, smtp_host: "", smtp_port: 587, smtp_user: "", smtp_password: "", recipient: "" },
  schedule: { enabled: false, frequency: "weekly", days: ["MON"], time: "08:00", topics: [] },
  conflict_profiles: [],
  conflict_settings: { max_papers_per_source: 200, scan_days_back: 30, competition_threshold: 30, auto_scan_with_newsletter: false },
};

function renderHome() {
  return render(
    <GenerationProvider>
      <ConflictProvider>
        <Home />
      </ConflictProvider>
    </GenerationProvider>
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  mockListen.mockResolvedValue(() => {});
  // Both GenerationProvider and ConflictProvider call get_config on mount.
  // Default: return mockConfig for get_config, throw for anything else.
  mockInvoke.mockImplementation(async (cmd: string) => {
    if (cmd === "get_config") return mockConfig;
    throw new Error(`Unexpected command: ${cmd}`);
  });
});

describe("Home page", () => {
  it("renders generate button", async () => {
    renderHome();
    expect(screen.getByRole("button", { name: /generate/i })).toBeInTheDocument();
  });

  it("renders Add Topic button", async () => {
    renderHome();
    expect(screen.getByRole("button", { name: /add topic/i })).toBeInTheDocument();
  });

  it("shows error when generating with empty topic", async () => {
    renderHome();
    await waitFor(() => screen.getByRole("button", { name: /generate/i }));
    fireEvent.click(screen.getByRole("button", { name: /generate/i }));
    await waitFor(() => {
      expect(screen.getByText(/please enter at least one topic/i)).toBeInTheDocument();
    });
  });

  it("does not call listen when topic is empty", async () => {
    renderHome();
    await waitFor(() => screen.getByRole("button", { name: /generate/i }));
    fireEvent.click(screen.getByRole("button", { name: /generate/i }));
    expect(mockListen).not.toHaveBeenCalled();
  });

  it("sets up event listener when generating", async () => {
    // Override: get_config returns config, generate_newsletter returns results
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "get_config") return mockConfig;
      if (cmd === "generate_newsletter") return [{ topic: "test", title: "Test", path: "/test.md", error: null }];
      throw new Error(`Unexpected command: ${cmd}`);
    });
    renderHome();
    await waitFor(() => mockInvoke.mock.calls.length > 0);

    const textarea = document.querySelector("textarea");
    if (textarea) {
      fireEvent.change(textarea, { target: { value: "machine learning" } });
      fireEvent.click(screen.getByRole("button", { name: /generate/i }));
      await waitFor(() => expect(mockListen).toHaveBeenCalled());
    }
  });

  it("resets running state and shows error if listen throws", async () => {
    mockListen.mockRejectedValueOnce(
      new Error("event.listen not allowed. Permissions: core:event:allow-listen")
    );
    renderHome();
    await waitFor(() => mockInvoke.mock.calls.length > 0);

    const textarea = document.querySelector("textarea");
    expect(textarea).not.toBeNull();
    fireEvent.change(textarea!, { target: { value: "machine learning" } });
    fireEvent.click(screen.getByRole("button", { name: /generate/i }));

    await waitFor(() => {
      expect(screen.getByText(/event\.listen not allowed/i)).toBeInTheDocument();
    });
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /generate/i })).not.toBeDisabled();
    });
  });

  it("shows fatal error in log when invoke rejects", async () => {
    // Override: get_config returns config, generate_newsletter rejects
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "get_config") return mockConfig;
      if (cmd === "generate_newsletter") throw new Error("Command failed: no papers found");
      throw new Error(`Unexpected command: ${cmd}`);
    });
    renderHome();
    await waitFor(() => mockInvoke.mock.calls.length > 0);

    const textarea = document.querySelector("textarea");
    fireEvent.change(textarea!, { target: { value: "bayesian stats" } });
    fireEvent.click(screen.getByRole("button", { name: /generate/i }));

    await waitFor(() => {
      expect(screen.getByText(/Command failed/i)).toBeInTheDocument();
    });
  });
});
