export type Preset = "icon" | "logo" | "scan" | "default";
export type ContourMode = "pixel" | "subpixel" | "scalar";
export type CurveMode = "polygon" | "smooth" | "spline" | "fit" | "potrace";
export type AlphaBackground = "white" | "black";
export type PreviewBackground = "transparent" | "white" | "black" | "gray";

export interface TraceControls {
  preset: Preset;
  contourMode: ContourMode;
  curveMode: CurveMode;
  thresholdMode: "auto" | "fixed";
  threshold: number;
  invert: boolean;
  alphaBackground: AlphaBackground;
  turdSize: number;
  optTolerance: number;
  optimizeIcon: boolean;
  isolateForeground: boolean;
  previewBackground: PreviewBackground;
  zoom: number;
}

export interface HistoryItem {
  id: string;
  name: string;
  createdAt: number;
  sourceDataUrl: string;
  sourceType: string;
  svg: string;
  controls: TraceControls;
  error?: string;
}
