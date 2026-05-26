import { render, screen } from "@testing-library/react";
import ProgressLog from "./ProgressLog";
import type { LogEntry } from "../lib/types";

const makeEntry = (
  message: string,
  type: LogEntry["type"] = "info",
  topic = ""
): LogEntry => ({
  ts: "12:00:00",
  topic,
  message,
  type,
});

describe("ProgressLog", () => {
  it("renders nothing when empty and not running", () => {
    const { container } = render(
      <ProgressLog entries={[]} running={false} />
    );
    expect(container.firstChild).toBeNull();
  });

  it("renders spinner when running with no entries", () => {
    render(<ProgressLog entries={[]} running={true} />);
    expect(screen.getByText("Generating…")).toBeInTheDocument();
  });

  it("renders log entries", () => {
    const entries = [
      makeEntry("Searching OpenAlex…"),
      makeEntry("Found 25 papers", "info"),
    ];
    render(<ProgressLog entries={entries} running={false} />);
    expect(screen.getByText("Searching OpenAlex…")).toBeInTheDocument();
    expect(screen.getByText("Found 25 papers")).toBeInTheDocument();
  });

  it("shows Complete header when done", () => {
    render(
      <ProgressLog entries={[makeEntry("done")]} running={false} />
    );
    expect(screen.getByText("Complete")).toBeInTheDocument();
  });

  it("shows entry count", () => {
    const entries = [makeEntry("a"), makeEntry("b"), makeEntry("c")];
    render(<ProgressLog entries={entries} running={false} />);
    expect(screen.getByText("3 events")).toBeInTheDocument();
  });

  it("applies error styling to error entries", () => {
    const entry = makeEntry("Something went wrong", "error");
    const { container } = render(
      <ProgressLog entries={[entry]} running={false} />
    );
    const errorRow = container.querySelector(".text-rust");
    expect(errorRow).toBeInTheDocument();
  });

  it("applies success styling to success entries", () => {
    const entry = makeEntry("All done!", "success");
    const { container } = render(
      <ProgressLog entries={[entry]} running={false} />
    );
    const successRow = container.querySelector(".text-success");
    expect(successRow).toBeInTheDocument();
  });

  it("shows topic label when topic is present", () => {
    const entry = makeEntry("Searching…", "info", "small area estimation");
    render(<ProgressLog entries={[entry]} running={false} />);
    expect(screen.getByText(/small area/)).toBeInTheDocument();
  });

  it("highlights streaming entries with the pine accent and pulsing icon", () => {
    const streaming: LogEntry = {
      ts: "12:00:01",
      topic: "deep learning",
      message: "Local model: 175 tok @ 8.4 tok/s",
      type: "info",
      streamingStep: "summarize",
      tokens_generated: 175,
      tokens_per_sec: 8.4,
      elapsed_ms: 21000,
    };
    const { container } = render(
      <ProgressLog entries={[streaming]} running={true} />
    );
    expect(
      screen.getByText("Local model: 175 tok @ 8.4 tok/s")
    ).toBeInTheDocument();
    // Pine-tinted row + a pulsing Activity icon mark this as live.
    expect(container.querySelector(".text-pine")).toBeInTheDocument();
    expect(container.querySelector(".animate-pulse")).toBeInTheDocument();
  });

  it("does not flag the streaming entry when a later non-streaming entry follows it", () => {
    const streaming: LogEntry = {
      ts: "12:00:01",
      topic: "x",
      message: "Local model: 50 tok @ 3 tok/s",
      type: "info",
      streamingStep: "summarize",
      tokens_generated: 50,
      tokens_per_sec: 3,
    };
    const finished: LogEntry = {
      ts: "12:00:05",
      topic: "x",
      message: "Saved newsletter",
      type: "success",
    };
    const { container } = render(
      <ProgressLog entries={[streaming, finished]} running={false} />
    );
    // The streaming entry should *not* be highlighted any more — it's
    // not the last entry. Only success/info coloring applies.
    const pineEls = container.querySelectorAll(".text-pine");
    expect(pineEls.length).toBe(0);
  });
});
