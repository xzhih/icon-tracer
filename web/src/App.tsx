import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { HistoryRail } from "./components/HistoryRail";
import { Inspector } from "./components/Inspector";
import { PreviewCanvas } from "./components/PreviewCanvas";
import { TopBar } from "./components/TopBar";
import { languageStorageKey, loadLanguage, messages, type Language } from "./i18n";
import { defaultControls, normalizeControls, toWasmOptions } from "./options";
import {
  dataUrlToBytes,
  fileToDataUrl,
  historyStorageKey,
  loadHistory,
  stripExtension,
} from "./storage";
import { isTraceCanceled, TraceWorkerRunner } from "./traceWorkerRunner";
import type { HistoryItem, TraceControls } from "./types";

function App() {
  const initialHistoryRef = useRef<HistoryItem[] | null>(null);
  if (!initialHistoryRef.current) {
    initialHistoryRef.current = loadHistory();
  }

  const [wasmReady, setWasmReady] = useState(false);
  const [language, setLanguage] = useState<Language>(loadLanguage);
  const [history, setHistory] = useState<HistoryItem[]>(() => initialHistoryRef.current ?? []);
  const [selectedId, setSelectedId] = useState<string | null>(
    () => initialHistoryRef.current?.[0]?.id ?? null,
  );
  const [dragActive, setDragActive] = useState(false);
  const [isTracing, setIsTracing] = useState(false);
  const [copied, setCopied] = useState(false);
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const traceRunRef = useRef(0);
  const traceWorkerRef = useRef<TraceWorkerRunner | null>(null);

  const selectedItem = useMemo(
    () => history.find((item) => item.id === selectedId) ?? null,
    [history, selectedId],
  );
  const controls = selectedItem?.controls ?? defaultControls;
  const selectedTraceOptionsKey = useMemo(
    () => (selectedItem ? JSON.stringify(toWasmOptions(selectedItem.controls)) : ""),
    [selectedItem?.controls],
  );
  const t = messages[language];

  useEffect(() => {
    const traceWorker = new TraceWorkerRunner();
    traceWorkerRef.current = traceWorker;
    setWasmReady(true);

    return () => {
      traceWorker.terminate();
      if (traceWorkerRef.current === traceWorker) {
        traceWorkerRef.current = null;
      }
    };
  }, []);

  useEffect(() => {
    localStorage.setItem(historyStorageKey, JSON.stringify(history.slice(0, 16)));
  }, [history]);

  useEffect(() => {
    document.documentElement.lang = language === "zh" ? "zh-CN" : "en";
    localStorage.setItem(languageStorageKey, language);
  }, [language]);

  const traceItem = useCallback(
    async (item: HistoryItem, nextControls: TraceControls) => {
      const traceWorker = traceWorkerRef.current;
      if (!wasmReady || !traceWorker) {
        return;
      }

      const runId = ++traceRunRef.current;
      setIsTracing(true);

      try {
        const bytes = await dataUrlToBytes(item.sourceDataUrl);
        const svg = await traceWorker.trace(bytes, toWasmOptions(nextControls));
        if (runId !== traceRunRef.current) {
          return;
        }
        setHistory((items) =>
          items.map((candidate) =>
            candidate.id === item.id
              ? { ...candidate, controls: nextControls, svg, error: undefined }
              : candidate,
          ),
        );
      } catch (error) {
        if (isTraceCanceled(error)) {
          return;
        }
        if (runId !== traceRunRef.current) {
          return;
        }
        setHistory((items) =>
          items.map((candidate) =>
            candidate.id === item.id
              ? {
                  ...candidate,
                  controls: nextControls,
                  error: error instanceof Error ? error.message : String(error),
                }
              : candidate,
          ),
        );
      } finally {
        if (runId === traceRunRef.current) {
          setIsTracing(false);
        }
      }
    },
    [wasmReady],
  );

  useEffect(() => {
    if (!selectedItem || !wasmReady) {
      return;
    }

    const handle = window.setTimeout(() => {
      void traceItem(selectedItem, selectedItem.controls);
    }, 120);

    return () => window.clearTimeout(handle);
  }, [selectedItem?.id, selectedTraceOptionsKey, wasmReady, traceItem]);

  const updateControls = (patch: Partial<TraceControls>) => {
    if (!selectedItem) {
      return;
    }

    const nextControls = normalizeControls({ ...selectedItem.controls, ...patch });
    setHistory((items) =>
      items.map((item) =>
        item.id === selectedItem.id ? { ...item, controls: nextControls } : item,
      ),
    );
  };

  const handleFiles = async (files: FileList | File[]) => {
    const file = Array.from(files).find((candidate) => candidate.size > 0);
    if (!file) {
      return;
    }

    const sourceDataUrl = await fileToDataUrl(file);
    const item: HistoryItem = {
      id: crypto.randomUUID(),
      name: file.name,
      createdAt: Date.now(),
      sourceDataUrl,
      sourceType: file.type,
      svg: "",
      controls,
    };

    setHistory((items) => [item, ...items].slice(0, 16));
    setSelectedId(item.id);
    if (wasmReady) {
      void traceItem(item, item.controls);
    }
  };

  const deleteItem = (id: string) => {
    const nextHistory = history.filter((item) => item.id !== id);
    setHistory(nextHistory);
    if (id === selectedId) {
      setSelectedId(nextHistory[0]?.id ?? null);
    }
  };

  const clearSelected = () => {
    if (!selectedItem) {
      return;
    }

    deleteItem(selectedItem.id);
  };

  const copySvg = async () => {
    if (!selectedItem?.svg) {
      return;
    }

    await navigator.clipboard.writeText(selectedItem.svg);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1200);
  };

  const downloadSvg = () => {
    if (!selectedItem?.svg) {
      return;
    }

    const blob = new Blob([selectedItem.svg], { type: "image/svg+xml" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `${stripExtension(selectedItem.name)}.svg`;
    anchor.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div
      className={`app-shell ${dragActive ? "is-dragging" : ""}`}
      onDragEnter={(event) => {
        event.preventDefault();
        setDragActive(true);
      }}
      onDragOver={(event) => event.preventDefault()}
      onDragLeave={(event) => {
        if (event.currentTarget === event.target) {
          setDragActive(false);
        }
      }}
      onDrop={(event) => {
        event.preventDefault();
        setDragActive(false);
        void handleFiles(event.dataTransfer.files);
      }}
    >
      <input
        ref={fileInputRef}
        className="file-input"
        type="file"
        accept="image/png,image/jpeg,image/bmp,.bmp,.pbm,.pgm,.ppm,.pnm"
        onChange={(event) => {
          if (event.target.files) {
            void handleFiles(event.target.files);
          }
          event.currentTarget.value = "";
        }}
      />

      <PreviewCanvas
        item={selectedItem}
        controls={controls}
        t={t}
        onUpload={() => fileInputRef.current?.click()}
        onZoomChange={(zoom) => updateControls({ zoom })}
      />

      <HistoryRail
        history={history}
        selectedId={selectedId}
        onSelect={setSelectedId}
        onDelete={deleteItem}
        t={t}
        onUpload={() => fileInputRef.current?.click()}
      />

      <TopBar
        item={selectedItem}
        isTracing={isTracing}
        copied={copied}
        language={language}
        t={t}
        onLanguageChange={setLanguage}
        onCopy={() => void copySvg()}
        onDownload={downloadSvg}
      />

      <Inspector
        controls={controls}
        disabled={!selectedItem}
        t={t}
        onChange={updateControls}
        onDelete={clearSelected}
      />
    </div>
  );
}

export default App;
