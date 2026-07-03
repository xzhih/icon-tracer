import type { ContourMode, CurveMode, Preset, TraceControls } from "./types";

export interface WasmTraceOptions {
  preset: Preset;
  threshold: string;
  invert: boolean;
  alphaBackground: string;
  contourMode: ContourMode;
  curveMode: CurveMode;
  turdSize: number;
  optTolerance: number;
  optimizeIcon: boolean;
  isolateForeground: boolean;
  pixelPotrace: boolean;
}

export const defaultControls: TraceControls = {
  preset: "icon",
  contourMode: "subpixel",
  curveMode: "potrace",
  thresholdMode: "auto",
  threshold: 180,
  invert: true,
  alphaBackground: "black",
  turdSize: 2,
  optTolerance: 0.75,
  optimizeIcon: false,
  isolateForeground: false,
  previewBackground: "white",
  zoom: 100,
};

export const presets: Array<{ value: Preset; label: string }> = [
  { value: "icon", label: "Icon" },
  { value: "logo", label: "Logo" },
  { value: "scan", label: "Scan" },
  { value: "default", label: "Default" },
];

export const contourModes: Array<{ value: ContourMode; label: string }> = [
  { value: "subpixel", label: "Subpixel" },
  { value: "scalar", label: "Scalar" },
  { value: "pixel", label: "Pixel" },
];

export const curveModes: Array<{ value: CurveMode; label: string }> = [
  { value: "potrace", label: "Potrace" },
  { value: "fit", label: "Fit" },
  { value: "spline", label: "Spline" },
  { value: "smooth", label: "Smooth" },
  { value: "polygon", label: "Polygon" },
];

export function toWasmOptions(controls: TraceControls): WasmTraceOptions {
  return {
    preset: controls.preset,
    threshold:
      controls.thresholdMode === "auto" ? "auto" : String(Math.round(controls.threshold)),
    invert: controls.invert,
    alphaBackground: controls.alphaBackground,
    contourMode: controls.contourMode,
    curveMode: controls.curveMode,
    turdSize: controls.turdSize,
    optTolerance: controls.optTolerance,
    optimizeIcon: controls.optimizeIcon,
    isolateForeground: controls.isolateForeground,
    // Pixel-potrace can take tens of seconds on 1024px icons; keep the web workbench interactive.
    pixelPotrace: false,
  };
}

export function normalizeControls(controls: TraceControls): TraceControls {
  return {
    ...controls,
    turdSize: clamp(Math.round(controls.turdSize), 0, 32),
    threshold: clamp(Math.round(controls.threshold), 0, 255),
    optTolerance: clamp(controls.optTolerance, 0, 2),
    zoom: clamp(Math.round(controls.zoom * 10) / 10, 25, 600),
    isolateForeground: controls.optimizeIcon ? controls.isolateForeground : false,
  };
}

export function presetDefaults(preset: Preset): Partial<TraceControls> {
  switch (preset) {
    case "default":
      return { contourMode: "pixel", curveMode: "polygon", turdSize: 0, optTolerance: 0 };
    case "logo":
      return { contourMode: "subpixel", curveMode: "potrace", turdSize: 4, optTolerance: 0.75 };
    case "scan":
      return { contourMode: "pixel", curveMode: "polygon", turdSize: 2, optTolerance: 0 };
    case "icon":
      return { contourMode: "subpixel", curveMode: "potrace", turdSize: 2, optTolerance: 0.75 };
  }
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}
