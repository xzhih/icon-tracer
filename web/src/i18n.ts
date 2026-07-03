import type { AlphaBackground, ContourMode, CurveMode, Preset, PreviewBackground } from "./types";

export type Language = "en" | "zh";

export const languageStorageKey = "icon-tracer-web-language-v1";

export interface Messages {
  alpha: string;
  auto: string;
  cancel: string;
  confirmDelete: string;
  confirmDeleteIcon: (name: string) => string;
  contour: string;
  copySvg: string;
  curve: string;
  cutoff: string;
  deleteIcon: (name: string) => string;
  downloadSvg: string;
  dropUpload: string;
  fixed: string;
  geometry: string;
  history: string;
  iconCleanup: string;
  input: string;
  invert: string;
  isolate: string;
  languageSwitch: string;
  mode: string;
  openIcon: (name: string) => string;
  openRepository: string;
  optimize: string;
  preset: string;
  preview: string;
  remove: string;
  trace: string;
  tracing: string;
  threshold: string;
  tolerance: string;
  turd: string;
  uploadIcon: string;
  zoom: string;
  alphaBackgrounds: Record<AlphaBackground, string>;
  contours: Record<ContourMode, string>;
  curves: Record<CurveMode, string>;
  presets: Record<Preset, string>;
  previewBackgrounds: Record<PreviewBackground, string>;
}

export const messages: Record<Language, Messages> = {
  en: {
    alpha: "Transparent fill",
    auto: "Auto",
    cancel: "Cancel",
    confirmDelete: "Confirm remove",
    confirmDeleteIcon: (name) => `Confirm deleting ${name}`,
    contour: "Edge",
    copySvg: "Copy SVG",
    curve: "Path style",
    cutoff: "Black cutoff",
    deleteIcon: (name) => `Delete ${name}`,
    downloadSvg: "Download SVG",
    dropUpload: "Drop or upload an icon",
    fixed: "Fixed",
    geometry: "Shape",
    history: "Icon history",
    iconCleanup: "Auto cleanup",
    input: "Source",
    invert: "Invert colors",
    isolate: "Keep subject",
    languageSwitch: "Language",
    mode: "Type",
    openIcon: (name) => `Open ${name}`,
    openRepository: "Open GitHub repository",
    optimize: "Smart cleanup",
    preset: "Use case",
    preview: "Preview",
    remove: "Remove",
    trace: "Convert",
    tracing: "Converting",
    threshold: "Detection",
    tolerance: "Simplify",
    turd: "Remove dots",
    uploadIcon: "Upload icon",
    zoom: "Zoom",
    alphaBackgrounds: {
      black: "Black",
      white: "White",
    },
    contours: {
      pixel: "Pixel edge",
      scalar: "Brightness edge",
      subpixel: "Smooth edge",
    },
    curves: {
      fit: "Tight fit",
      polygon: "Straight edges",
      potrace: "Smooth trace",
      smooth: "Rounded",
      spline: "Spline",
    },
    presets: {
      default: "Raw",
      icon: "Icon",
      logo: "Logo",
      scan: "Scan art",
    },
    previewBackgrounds: {
      black: "Black preview background",
      gray: "Gray preview background",
      transparent: "Transparent preview background",
      white: "White preview background",
    },
  },
  zh: {
    alpha: "透明区域底色",
    auto: "自动",
    cancel: "取消",
    confirmDelete: "确认移除",
    confirmDeleteIcon: (name) => `确认删除 ${name}`,
    contour: "边缘",
    copySvg: "复制 SVG",
    curve: "线条",
    cutoff: "黑白分界",
    deleteIcon: (name) => `删除 ${name}`,
    downloadSvg: "下载 SVG",
    dropUpload: "拖入或上传图标",
    fixed: "固定",
    geometry: "形状",
    history: "图标历史",
    iconCleanup: "自动清理",
    input: "识别",
    invert: "反转黑白",
    isolate: "只保留主体",
    languageSwitch: "语言",
    mode: "类型",
    openIcon: (name) => `打开 ${name}`,
    openRepository: "打开 GitHub 仓库",
    optimize: "智能清理",
    preset: "用途",
    preview: "预览",
    remove: "移除",
    trace: "生成",
    tracing: "生成中",
    threshold: "识别方式",
    tolerance: "简化",
    turd: "去杂点",
    uploadIcon: "上传图标",
    zoom: "缩放",
    alphaBackgrounds: {
      black: "黑色",
      white: "白色",
    },
    contours: {
      pixel: "像素边缘",
      scalar: "亮度边缘",
      subpixel: "平滑边缘",
    },
    curves: {
      fit: "贴合边缘",
      polygon: "直线折角",
      potrace: "平滑描线",
      smooth: "圆滑",
      spline: "样条曲线",
    },
    presets: {
      default: "原始",
      icon: "图标",
      logo: "Logo",
      scan: "扫描图",
    },
    previewBackgrounds: {
      black: "黑色预览背景",
      gray: "灰色预览背景",
      transparent: "透明预览背景",
      white: "白色预览背景",
    },
  },
};

export function loadLanguage(): Language {
  try {
    const stored = localStorage.getItem(languageStorageKey);
    if (stored === "en" || stored === "zh") {
      return stored;
    }
  } catch {
    // Ignore storage failures and fall back to the browser locale.
  }

  return navigator.language.toLowerCase().startsWith("zh") ? "zh" : "en";
}
