import { ImagePlus } from "lucide-react";
import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type PointerEvent,
} from "react";
import type { Messages } from "../i18n";
import type { HistoryItem, TraceControls } from "../types";

const minCanvasZoom = 25;
const maxCanvasZoom = 600;
const wheelLineHeightPx = 16;
const wheelZoomSensitivity = 0.006;
const maxWheelZoomFactor = 1.45;

interface PreviewCanvasProps {
  item: HistoryItem | null;
  controls: TraceControls;
  t: Messages;
  onUpload: () => void;
  onZoomChange: (zoom: number) => void;
}

interface PanState {
  pointerId: number;
  startX: number;
  startY: number;
  scrollLeft: number;
  scrollTop: number;
}

interface ZoomFocus {
  contentX: number;
  contentY: number;
  pointerX: number;
  pointerY: number;
  ratio: number;
}

export function PreviewCanvas({
  item,
  controls,
  t,
  onUpload,
  onZoomChange,
}: PreviewCanvasProps) {
  const stageRef = useRef<HTMLElement | null>(null);
  const viewportRef = useRef<HTMLDivElement | null>(null);
  const panRef = useRef<PanState | null>(null);
  const zoomFocusRef = useRef<ZoomFocus | null>(null);
  const [isPanning, setIsPanning] = useState(false);

  useLayoutEffect(() => {
    const focus = zoomFocusRef.current;
    const viewport = viewportRef.current;
    if (!focus || !viewport) {
      return;
    }

    viewport.scrollLeft = focus.contentX * focus.ratio - focus.pointerX;
    viewport.scrollTop = focus.contentY * focus.ratio - focus.pointerY;
    zoomFocusRef.current = null;
  }, [controls.zoom]);

  useEffect(() => {
    const stage = stageRef.current;
    if (!stage) {
      return undefined;
    }

    const handleWheel = (event: WheelEvent) => {
      if (!isZoomWheelEvent(event)) {
        return;
      }

      event.preventDefault();

      if (!item?.svg) {
        return;
      }

      const viewport = viewportRef.current;
      if (!viewport) {
        return;
      }

      const rect = viewport.getBoundingClientRect();
      const pointerX = event.clientX - rect.left;
      const pointerY = event.clientY - rect.top;
      const contentX = viewport.scrollLeft + pointerX;
      const contentY = viewport.scrollTop + pointerY;
      const deltaY = event.deltaMode === 1 ? event.deltaY * wheelLineHeightPx : event.deltaY;
      const nextZoom = clampZoom(controls.zoom * getWheelZoomFactor(deltaY));

      if (nextZoom === controls.zoom) {
        return;
      }

      const ratio = nextZoom / controls.zoom;
      zoomFocusRef.current = { contentX, contentY, pointerX, pointerY, ratio };
      onZoomChange(nextZoom);
    };

    stage.addEventListener("wheel", handleWheel, { passive: false });
    return () => stage.removeEventListener("wheel", handleWheel);
  }, [controls.zoom, item?.svg, onZoomChange]);

  const startPan = (event: PointerEvent<HTMLDivElement>) => {
    if (!item?.svg || event.button !== 0) {
      return;
    }

    const viewport = viewportRef.current;
    if (!viewport) {
      return;
    }

    panRef.current = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      scrollLeft: viewport.scrollLeft,
      scrollTop: viewport.scrollTop,
    };
    viewport.setPointerCapture(event.pointerId);
    setIsPanning(true);
  };

  const movePan = (event: PointerEvent<HTMLDivElement>) => {
    const pan = panRef.current;
    const viewport = viewportRef.current;
    if (!pan || !viewport || pan.pointerId !== event.pointerId) {
      return;
    }

    viewport.scrollLeft = pan.scrollLeft - (event.clientX - pan.startX);
    viewport.scrollTop = pan.scrollTop - (event.clientY - pan.startY);
  };

  const stopPan = (event: PointerEvent<HTMLDivElement>) => {
    const pan = panRef.current;
    const viewport = viewportRef.current;
    if (!pan || pan.pointerId !== event.pointerId) {
      return;
    }

    if (viewport?.hasPointerCapture(event.pointerId)) {
      viewport.releasePointerCapture(event.pointerId);
    }
    panRef.current = null;
    setIsPanning(false);
  };

  return (
    <section ref={stageRef} className={`preview-stage bg-${controls.previewBackground}`}>
      {item?.svg ? (
        <div
          ref={viewportRef}
          className={`preview-scroll ${isPanning ? "is-panning" : ""}`}
          onPointerDown={startPan}
          onPointerMove={movePan}
          onPointerUp={stopPan}
          onPointerCancel={stopPan}
        >
          <div
            className="preview-content"
            style={{ "--preview-zoom": controls.zoom / 100 } as CSSProperties}
          >
            <div className="svg-preview" dangerouslySetInnerHTML={{ __html: item.svg }} />
          </div>
        </div>
      ) : (
        <div className="empty-preview">
          <button className="drop-target" type="button" onClick={onUpload}>
            <ImagePlus size={42} />
            <span>{t.dropUpload}</span>
          </button>
        </div>
      )}
      {item?.error ? <div className="error-banner">{item.error}</div> : null}
    </section>
  );
}

function clampZoom(zoom: number): number {
  return Math.min(maxCanvasZoom, Math.max(minCanvasZoom, Math.round(zoom * 10) / 10));
}

function getWheelZoomFactor(deltaY: number): number {
  return Math.min(
    maxWheelZoomFactor,
    Math.max(1 / maxWheelZoomFactor, Math.exp(-deltaY * wheelZoomSensitivity)),
  );
}

function isZoomWheelEvent(event: WheelEvent): boolean {
  // Chromium exposes trackpad pinch as a ctrlKey wheel event; metaKey keeps Cmd-wheel useful on macOS.
  return event.ctrlKey || event.metaKey;
}
