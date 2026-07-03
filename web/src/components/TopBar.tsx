import { Check, Clipboard, Download, Github, Loader2 } from "lucide-react";
import type { Language, Messages } from "../i18n";
import type { HistoryItem } from "../types";

const repositoryUrl = "https://github.com/xzhih/icon-tracer";

interface TopBarProps {
  item: HistoryItem | null;
  isTracing: boolean;
  copied: boolean;
  language: Language;
  t: Messages;
  onLanguageChange: (language: Language) => void;
  onCopy: () => void;
  onDownload: () => void;
}

export function TopBar({
  item,
  isTracing,
  copied,
  language,
  t,
  onLanguageChange,
  onCopy,
  onDownload,
}: TopBarProps) {
  return (
    <header className="top-bar">
      <a
        className="topbar-github"
        href={repositoryUrl}
        target="_blank"
        rel="noreferrer"
        aria-label={t.openRepository}
        title={t.openRepository}
      >
        <Github size={18} />
      </a>
      <div className="toolbar">
        <div className="language-switch" role="group" aria-label={t.languageSwitch}>
          <button
            className={`language-choice ${language === "en" ? "is-selected" : ""}`}
            type="button"
            onClick={() => onLanguageChange("en")}
            aria-pressed={language === "en"}
          >
            EN
          </button>
          <button
            className={`language-choice ${language === "zh" ? "is-selected" : ""}`}
            type="button"
            onClick={() => onLanguageChange("zh")}
            aria-pressed={language === "zh"}
          >
            中
          </button>
        </div>
        <button
          className="tool-button"
          type="button"
          onClick={onCopy}
          disabled={!item?.svg}
          aria-label={t.copySvg}
        >
          {copied ? <Check size={18} /> : <Clipboard size={18} />}
        </button>
        <button
          className="tool-button"
          type="button"
          onClick={onDownload}
          disabled={!item?.svg}
          aria-label={t.downloadSvg}
        >
          <Download size={18} />
        </button>
        {isTracing ? (
          <span className="trace-status is-active">
            <Loader2 size={15} />
            <span>{t.tracing}</span>
          </span>
        ) : null}
      </div>
    </header>
  );
}
