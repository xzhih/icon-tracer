import { useId, useState, type ReactNode } from "react";
import { ChevronDown, Trash2 } from "lucide-react";
import type { Messages } from "../i18n";
import { contourModes, curveModes, presetDefaults, presets } from "../options";
import type { AlphaBackground, PreviewBackground, TraceControls } from "../types";

interface InspectorProps {
  controls: TraceControls;
  disabled: boolean;
  t: Messages;
  onChange: (patch: Partial<TraceControls>) => void;
  onDelete: () => void;
}

export function Inspector({ controls, disabled, t, onChange, onDelete }: InspectorProps) {
  const [activeTab, setActiveTab] = useState<"trace" | "preview">("trace");
  const presetOptions = presets.map((option) => ({
    ...option,
    label: t.presets[option.value],
  }));
  const contourOptions = contourModes.map((option) => ({
    ...option,
    label: t.contours[option.value],
  }));
  const curveOptions = curveModes.map((option) => ({
    ...option,
    label: t.curves[option.value],
  }));
  const thresholdOptions: Array<{ value: TraceControls["thresholdMode"]; label: string }> = [
    { value: "auto", label: t.auto },
    { value: "fixed", label: t.fixed },
  ];
  const alphaOptions: Array<{ value: AlphaBackground; label: string }> = [
    { value: "white", label: t.alphaBackgrounds.white },
    { value: "black", label: t.alphaBackgrounds.black },
  ];

  return (
    <aside className="inspector">
      <div className="inspector-tabs" role="tablist" aria-label={t.trace}>
        <button
          className={`inspector-tab ${activeTab === "trace" ? "is-selected" : ""}`}
          type="button"
          role="tab"
          aria-selected={activeTab === "trace"}
          onClick={() => setActiveTab("trace")}
        >
          {t.trace}
        </button>
        <button
          className={`inspector-tab ${activeTab === "preview" ? "is-selected" : ""}`}
          type="button"
          role="tab"
          aria-selected={activeTab === "preview"}
          onClick={() => setActiveTab("preview")}
        >
          {t.preview}
        </button>
      </div>

      {activeTab === "trace" ? (
        <>
          <Panel title={t.preset}>
            <SelectRow
              label={t.mode}
              value={controls.preset}
              disabled={disabled}
              options={presetOptions}
              onChange={(preset) => onChange({ ...presetDefaults(preset), preset })}
            />
          </Panel>

          <Panel title={t.input}>
            <SelectRow
              label={t.threshold}
              value={controls.thresholdMode}
              disabled={disabled}
              options={thresholdOptions}
              onChange={(thresholdMode) => onChange({ thresholdMode })}
            />
            {controls.thresholdMode === "fixed" ? (
              <SliderRow
                label={t.cutoff}
                value={controls.threshold}
                min={0}
                max={255}
                step={1}
                disabled={disabled}
                onChange={(threshold) => onChange({ threshold })}
              />
            ) : null}
            <ToggleRow
              label={t.invert}
              checked={controls.invert}
              disabled={disabled}
              onChange={(invert) => onChange({ invert })}
            />
            <SelectRow
              label={t.alpha}
              value={controls.alphaBackground}
              disabled={disabled}
              options={alphaOptions}
              onChange={(alphaBackground) => onChange({ alphaBackground })}
            />
          </Panel>

          <Panel title={t.geometry}>
            <SelectRow
              label={t.contour}
              value={controls.contourMode}
              disabled={disabled}
              options={contourOptions}
              onChange={(contourMode) => onChange({ contourMode })}
            />
            <SelectRow
              label={t.curve}
              value={controls.curveMode}
              disabled={disabled}
              options={curveOptions}
              onChange={(curveMode) => onChange({ curveMode })}
            />
            <StepperRow
              label={t.turd}
              value={controls.turdSize}
              min={0}
              max={32}
              disabled={disabled}
              onChange={(turdSize) => onChange({ turdSize })}
            />
            <SliderRow
              label={t.tolerance}
              value={controls.optTolerance}
              min={0}
              max={2}
              step={0.05}
              disabled={disabled}
              onChange={(optTolerance) => onChange({ optTolerance })}
            />
          </Panel>

          <Panel title={t.iconCleanup}>
            <ToggleRow
              label={t.optimize}
              checked={controls.optimizeIcon}
              disabled={disabled}
              onChange={(optimizeIcon) => onChange({ optimizeIcon })}
            />
            {controls.optimizeIcon ? (
              <ToggleRow
                label={t.isolate}
                checked={controls.isolateForeground}
                disabled={disabled}
                onChange={(isolateForeground) => onChange({ isolateForeground })}
              />
            ) : null}
          </Panel>
        </>
      ) : (
        <Panel title={t.preview}>
          <SwatchRow
            value={controls.previewBackground}
            disabled={disabled}
            t={t}
            onChange={(previewBackground) => onChange({ previewBackground })}
          />
          <SliderRow
            label={t.zoom}
            value={controls.zoom}
            min={25}
            max={600}
            step={5}
            disabled={disabled}
            suffix="%"
            onChange={(zoom) => onChange({ zoom })}
          />
        </Panel>
      )}

      <button className="danger-button" type="button" disabled={disabled} onClick={onDelete}>
        <Trash2 size={16} />
        <span>{t.remove}</span>
      </button>
    </aside>
  );
}

function Panel({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="panel">
      <h2>{title}</h2>
      <div className="panel-body">{children}</div>
    </section>
  );
}

function SelectRow<T extends string>({
  label,
  value,
  options,
  disabled,
  onChange,
}: {
  label: string;
  value: T;
  options: Array<{ value: T; label: string }>;
  disabled?: boolean;
  onChange: (value: T) => void;
}) {
  const [isOpen, setIsOpen] = useState(false);
  const listId = useId();
  const selectedOption = options.find((option) => option.value === value) ?? options[0];

  return (
    <div className="control-row">
      <span>{label}</span>
      <span
        className={`select-shell ${isOpen ? "is-open" : ""}`}
        onBlur={(event) => {
          const nextFocus = event.relatedTarget;
          if (!(nextFocus instanceof Node) || !event.currentTarget.contains(nextFocus)) {
            setIsOpen(false);
          }
        }}
      >
        <button
          className="select-button"
          type="button"
          disabled={disabled}
          aria-haspopup="listbox"
          aria-expanded={isOpen}
          aria-controls={listId}
          onClick={() => setIsOpen((open) => !open)}
          onKeyDown={(event) => {
            if (event.key === "Escape") {
              setIsOpen(false);
            }
          }}
        >
          <span>{selectedOption?.label}</span>
          <ChevronDown size={14} />
        </button>
        {isOpen ? (
          <div className="select-menu" id={listId} role="listbox">
            {options.map((option) => (
              <button
                key={option.value}
                className={`select-option ${option.value === value ? "is-selected" : ""}`}
                type="button"
                role="option"
                aria-selected={option.value === value}
                onClick={() => {
                  onChange(option.value);
                  setIsOpen(false);
                }}
              >
                {option.label}
              </button>
            ))}
          </div>
        ) : null}
      </span>
    </div>
  );
}

function SliderRow({
  label,
  value,
  min,
  max,
  step,
  suffix,
  disabled,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  suffix?: string;
  disabled?: boolean;
  onChange: (value: number) => void;
}) {
  return (
    <label className="slider-row">
      <span>
        {label}
        <strong>{formatSliderValue(value, suffix)}</strong>
      </span>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        disabled={disabled}
        onChange={(event) => onChange(Number(event.target.value))}
      />
    </label>
  );
}

function formatSliderValue(value: number, suffix?: string): string {
  const formatted = Number.isInteger(value)
    ? String(value)
    : Math.abs(value) >= 10
      ? value.toFixed(1)
      : value.toFixed(2);
  return `${formatted}${suffix ?? ""}`;
}

function StepperRow({
  label,
  value,
  min,
  max,
  disabled,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  disabled?: boolean;
  onChange: (value: number) => void;
}) {
  return (
    <label className="control-row">
      <span>{label}</span>
      <input
        className="number-input"
        type="number"
        min={min}
        max={max}
        value={value}
        disabled={disabled}
        onChange={(event) => onChange(Number(event.target.value))}
      />
    </label>
  );
}

function ToggleRow({
  label,
  checked,
  disabled,
  onChange,
}: {
  label: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <label className="control-row">
      <span>{label}</span>
      <input
        className="switch"
        type="checkbox"
        checked={checked}
        disabled={disabled}
        onChange={(event) => onChange(event.target.checked)}
      />
    </label>
  );
}

function SwatchRow({
  value,
  disabled,
  t,
  onChange,
}: {
  value: PreviewBackground;
  disabled?: boolean;
  t: Messages;
  onChange: (value: PreviewBackground) => void;
}) {
  const swatches: PreviewBackground[] = ["transparent", "black", "gray", "white"];
  return (
    <div className="swatch-row">
      {swatches.map((swatch) => (
        <button
          key={swatch}
          className={`swatch swatch-${swatch} ${value === swatch ? "is-selected" : ""}`}
          type="button"
          disabled={disabled}
          onClick={() => onChange(swatch)}
          aria-label={t.previewBackgrounds[swatch]}
        />
      ))}
    </div>
  );
}
