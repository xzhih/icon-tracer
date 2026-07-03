import type { WasmTraceOptions } from "./options";
import TraceWorker from "./traceWorker?worker";

interface ActiveTrace {
  reject: (error: Error) => void;
  worker: Worker;
}

type TraceWorkerResponse = { svg: string } | { error: string };

export class TraceWorkerRunner {
  private activeTrace: ActiveTrace | null = null;

  trace(bytes: Uint8Array, options: WasmTraceOptions): Promise<string> {
    this.cancelActiveTrace();

    const worker = new TraceWorker();
    const payload = bytesToTransferableBuffer(bytes);

    return new Promise((resolve, reject) => {
      this.activeTrace = { reject, worker };

      worker.onmessage = (event: MessageEvent<TraceWorkerResponse>) => {
        this.finish(worker);
        if ("svg" in event.data) {
          resolve(event.data.svg);
        } else {
          reject(new Error(event.data.error));
        }
      };

      worker.onerror = (event) => {
        this.finish(worker);
        reject(new Error(event.message));
      };

      worker.postMessage({ bytes: payload, options }, [payload]);
    });
  }

  terminate() {
    this.cancelActiveTrace();
  }

  private cancelActiveTrace() {
    if (!this.activeTrace) {
      return;
    }

    const { reject, worker } = this.activeTrace;
    this.activeTrace = null;
    worker.terminate();
    reject(new DOMException("Trace canceled", "AbortError"));
  }

  private finish(worker: Worker) {
    if (this.activeTrace?.worker === worker) {
      this.activeTrace = null;
    }
    worker.terminate();
  }
}

export function isTraceCanceled(error: unknown): boolean {
  return error instanceof DOMException && error.name === "AbortError";
}

function bytesToTransferableBuffer(bytes: Uint8Array): ArrayBuffer {
  if (bytes.byteOffset === 0 && bytes.byteLength === bytes.buffer.byteLength) {
    return bytes.buffer as ArrayBuffer;
  }

  return bytes.slice().buffer as ArrayBuffer;
}
