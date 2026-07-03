import { FileUp, ImagePlus, Trash2 } from "lucide-react";
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
  return (
    <aside className="history-rail" aria-label={t.history}>
      <div className="history-list">
        {history.map((item) => (
          <div
            key={item.id}
            className={`history-item ${item.id === selectedId ? "is-selected" : ""}`}
          >
            <button
              className="history-thumb"
              type="button"
              onClick={() => onSelect(item.id)}
              aria-label={t.openIcon(item.name)}
              title={item.name}
            >
              {item.sourceType.startsWith("image/") ? (
                <img src={item.sourceDataUrl} alt="" />
              ) : (
                <FileUp size={42} />
              )}
            </button>
            <button
              className="history-delete"
              type="button"
              onClick={(event) => {
                event.stopPropagation();
                onDelete(item.id);
              }}
              aria-label={t.deleteIcon(item.name)}
              title={t.remove}
            >
              <Trash2 size={18} />
            </button>
          </div>
        ))}
      </div>
      <button className="rail-upload" type="button" onClick={onUpload} aria-label={t.uploadIcon}>
        <ImagePlus size={22} />
      </button>
    </aside>
  );
}
