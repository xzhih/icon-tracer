import type { WasmTraceOptions } from "./options";
import initWasm, { trace_image_to_svg_wasm } from "./wasm-pkg/icon_tracer_wasm";

interface TraceWorkerRequest {
  bytes: ArrayBuffer;
  options: WasmTraceOptions;
}

type TraceWorkerResponse = { svg: string } | { error: string };

const wasmReady = initWasm();

self.onmessage = async (event: MessageEvent<TraceWorkerRequest>) => {
  try {
    await wasmReady;
    const svg = trace_image_to_svg_wasm(
      new Uint8Array(event.data.bytes),
      event.data.options,
    );
    self.postMessage({ svg } satisfies TraceWorkerResponse);
  } catch (error) {
    self.postMessage({
      error: error instanceof Error ? error.message : String(error),
    } satisfies TraceWorkerResponse);
  }
};
