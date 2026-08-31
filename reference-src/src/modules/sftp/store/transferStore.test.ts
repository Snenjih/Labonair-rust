import { beforeEach, describe, expect, it, vi } from "vitest";
import { useTransferStore, type TransferJob } from "./transferStore";

// Reset store state between tests
beforeEach(() => {
  useTransferStore.setState({ jobs: [], stickyConflictResolution: {} });
});

function makeJob(id: string, status: TransferJob["status"] = "queued"): TransferJob {
  return {
    id,
    session_id: "sess-1",
    src_path: "/src/file.txt",
    dest_path: "/dest/file.txt",
    direction: "upload",
    status,
    bytes_total: 1000,
    bytes_transferred: 0,
    speed_bps: 0,
    skipped_count: 0,
  };
}

describe("addJob", () => {
  it("adds a job to an empty list", () => {
    useTransferStore.getState().addJob(makeJob("job-1"));
    expect(useTransferStore.getState().jobs).toHaveLength(1);
  });

  it("prepends new jobs (newest first)", () => {
    useTransferStore.getState().addJob(makeJob("job-1"));
    useTransferStore.getState().addJob(makeJob("job-2"));
    const { jobs } = useTransferStore.getState();
    expect(jobs[0].id).toBe("job-2");
    expect(jobs[1].id).toBe("job-1");
  });
});

describe("updateJob", () => {
  it("merges updated fields into the existing job", () => {
    useTransferStore.getState().addJob(makeJob("job-1"));
    useTransferStore.getState().updateJob({
      ...makeJob("job-1", "running"),
      bytes_transferred: 500,
    });
    const job = useTransferStore.getState().jobs.find((j) => j.id === "job-1");
    expect(job?.status).toBe("running");
    expect(job?.bytes_transferred).toBe(500);
  });

  it("does not affect other jobs", () => {
    useTransferStore.getState().addJob(makeJob("job-1"));
    useTransferStore.getState().addJob(makeJob("job-2"));
    useTransferStore.getState().updateJob({ ...makeJob("job-1", "completed") });
    const job2 = useTransferStore.getState().jobs.find((j) => j.id === "job-2");
    expect(job2?.status).toBe("queued");
  });
});

describe("removeJob", () => {
  it("removes a job by id", () => {
    useTransferStore.getState().addJob(makeJob("job-1"));
    useTransferStore.getState().addJob(makeJob("job-2"));
    useTransferStore.getState().removeJob("job-1");
    const { jobs } = useTransferStore.getState();
    expect(jobs).toHaveLength(1);
    expect(jobs[0].id).toBe("job-2");
  });

  it("does nothing when job not found", () => {
    useTransferStore.getState().addJob(makeJob("job-1"));
    useTransferStore.getState().removeJob("non-existent");
    expect(useTransferStore.getState().jobs).toHaveLength(1);
  });
});

describe("clearCompleted", () => {
  it("removes completed and cancelled jobs", () => {
    useTransferStore.getState().addJob(makeJob("j1", "completed"));
    useTransferStore.getState().addJob(makeJob("j2", "cancelled"));
    useTransferStore.getState().addJob(makeJob("j3", "running"));
    useTransferStore.getState().addJob(makeJob("j4", "queued"));
    useTransferStore.getState().clearCompleted();
    const { jobs } = useTransferStore.getState();
    expect(jobs).toHaveLength(2);
    expect(jobs.map((j) => j.id)).toContain("j3");
    expect(jobs.map((j) => j.id)).toContain("j4");
  });

  it("keeps failed jobs", () => {
    useTransferStore.getState().addJob(makeJob("j1", { failed: "connection lost" }));
    useTransferStore.getState().clearCompleted();
    expect(useTransferStore.getState().jobs).toHaveLength(1);
  });

  it("handles empty list gracefully", () => {
    useTransferStore.getState().clearCompleted();
    expect(useTransferStore.getState().jobs).toHaveLength(0);
  });
});

describe("cancelJob", () => {
  it("calls invoke with cancel_transfer command", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockResolvedValue(undefined);
    await useTransferStore.getState().cancelJob("job-1");
    expect(invoke).toHaveBeenCalledWith("cancel_transfer", { jobId: "job-1" });
  });
});

describe("setStickyConflictResolution", () => {
  it("sets and clears a per-session sticky resolution", () => {
    useTransferStore.getState().setStickyConflictResolution("sess-1", "skip");
    expect(useTransferStore.getState().stickyConflictResolution["sess-1"]).toBe("skip");
    useTransferStore.getState().setStickyConflictResolution("sess-1", null);
    expect(useTransferStore.getState().stickyConflictResolution["sess-1"]).toBeUndefined();
  });

  it("keeps other sessions' sticky resolutions independent", () => {
    useTransferStore.getState().setStickyConflictResolution("sess-1", "overwrite");
    useTransferStore.getState().setStickyConflictResolution("sess-2", "skip");
    expect(useTransferStore.getState().stickyConflictResolution).toEqual({
      "sess-1": "overwrite",
      "sess-2": "skip",
    });
  });
});

describe("file_conflict listener — sticky auto-resolve", () => {
  // Regression coverage for Item 9: "Skip All"/"Overwrite All" must also
  // auto-resolve conflicts discovered progressively AFTER the sticky choice
  // was made (e.g. a large recursive folder copy reporting conflicts one at
  // a time), not just the already-visible batch. This exercises the actual
  // `file_conflict` event handler registered by `bootstrapTransferListeners`,
  // not just the store's own setter.
  async function getFileConflictHandler() {
    const { listen } = await import("@tauri-apps/api/event");
    const { bootstrapTransferListeners } = await import("./transferStore");
    await bootstrapTransferListeners();
    const call = vi.mocked(listen).mock.calls.find(([eventName]) => eventName === "file_conflict");
    if (!call) throw new Error("file_conflict listener was never registered");
    return call[1] as (event: { payload: { job_id: string; src_path: string; dest_path: string } }) => void;
  }

  it("pauses the job and surfaces the conflict when no sticky resolution is set", async () => {
    const handler = await getFileConflictHandler();
    useTransferStore.getState().addJob(makeJob("job-1"));
    handler({ payload: { job_id: "job-1", src_path: "/a", dest_path: "/b" } });
    const job = useTransferStore.getState().jobs.find((j) => j.id === "job-1");
    expect(job?.status).toBe("paused");
    expect(job?.conflict).toEqual({ src_path: "/a", dest_path: "/b" });
  });

  it("auto-resolves via invoke without ever pausing when a sticky resolution is set", async () => {
    const handler = await getFileConflictHandler();
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockClear().mockResolvedValue(undefined);

    useTransferStore.getState().addJob(makeJob("job-2"));
    useTransferStore.getState().setStickyConflictResolution("sess-1", "skip");
    handler({ payload: { job_id: "job-2", src_path: "/a", dest_path: "/b" } });

    // resolveConflict is async (invoke call) — flush microtasks.
    await Promise.resolve();
    await Promise.resolve();

    const job = useTransferStore.getState().jobs.find((j) => j.id === "job-2");
    expect(job?.status).toBe("running");
    expect(job?.conflict).toBeUndefined();
    expect(invoke).toHaveBeenCalledWith("resolve_conflict", {
      jobId: "job-2",
      resolution: "skip",
      newName: null,
    });
  });

  it("ignores an event for a job that no longer exists", async () => {
    const handler = await getFileConflictHandler();
    expect(() => handler({ payload: { job_id: "nope", src_path: "/a", dest_path: "/b" } })).not.toThrow();
  });
});
