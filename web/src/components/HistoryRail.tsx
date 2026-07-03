import { useEffect, useState } from "react";
import { Check, FileUp, ImagePlus, Trash2, X } from "lucide-react";
import type { Messages } from "../i18n";
import type { HistoryItem } from "../types";

interface HistoryRailProps {
  history: HistoryItem[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onDelete: (id: string) => void;
  t: Messages;
  onUpload: () => void;
}

export function HistoryRail({
  history,
  selectedId,
  onSelect,
  onDelete,
  t,
  onUpload,
}: HistoryRailProps) {
  const [pendingDeleteId, setPendingDeleteId] = useState<string | null>(null);

  useEffect(() => {
    if (pendingDeleteId && !history.some((item) => item.id === pendingDeleteId)) {
      setPendingDeleteId(null);
    }
  }, [history, pendingDeleteId]);

  return (
    <aside className="history-rail" aria-label={t.history}>
      <div className="history-list">
        {history.map((item) => {
          const isConfirmingDelete = item.id === pendingDeleteId;

          return (
            <div
              key={item.id}
              className={`history-item ${item.id === selectedId ? "is-selected" : ""}`}
            >
              <button
                className="history-thumb"
                type="button"
                onClick={() => {
                  setPendingDeleteId(null);
                  onSelect(item.id);
                }}
                aria-label={t.openIcon(item.name)}
                title={item.name}
              >
                {item.sourceType.startsWith("image/") ? (
                  <img src={item.sourceDataUrl} alt="" />
                ) : (
                  <FileUp size={42} />
                )}
              </button>
              {isConfirmingDelete ? (
                <div
                  className="history-confirm"
                  role="group"
                  aria-label={t.confirmDeleteIcon(item.name)}
                >
                  <button
                    className="history-confirm-button"
                    type="button"
                    onClick={(event) => {
                      event.stopPropagation();
                      setPendingDeleteId(null);
                    }}
                    aria-label={t.cancel}
                    title={t.cancel}
                  >
                    <X size={18} />
                  </button>
                  <button
                    className="history-confirm-button is-danger"
                    type="button"
                    onClick={(event) => {
                      event.stopPropagation();
                      onDelete(item.id);
                      setPendingDeleteId(null);
                    }}
                    aria-label={t.confirmDeleteIcon(item.name)}
                    title={t.confirmDelete}
                  >
                    <Check size={18} />
                  </button>
                </div>
              ) : (
                <button
                  className="history-delete"
                  type="button"
                  onClick={(event) => {
                    event.stopPropagation();
                    setPendingDeleteId(item.id);
                  }}
                  aria-label={t.deleteIcon(item.name)}
                  title={t.remove}
                >
                  <Trash2 size={18} />
                </button>
              )}
            </div>
          );
        })}
      </div>
      <button className="rail-upload" type="button" onClick={onUpload} aria-label={t.uploadIcon}>
        <ImagePlus size={22} />
      </button>
    </aside>
  );
}
