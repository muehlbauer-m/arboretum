import { useMemo, useState } from "react";
import {
  Check,
  ChevronDown,
  ChevronUp,
  Eye,
  EyeOff,
  Loader,
  Plug,
  AlertCircle,
  CheckCircle,
} from "lucide-react";
import type { EmailConfig } from "../lib/types";
import Serif from "./Serif";
import SmallCaps from "./SmallCaps";

interface Provider {
  id: "gmail" | "outlook" | "icloud" | "custom";
  name: string;
  host: string;
  port: number;
  addressPlaceholder: string;
  note: string;
  guideUrl?: string;
}

const PROVIDERS: Provider[] = [
  {
    id: "gmail",
    name: "Gmail",
    host: "smtp.gmail.com",
    port: 587,
    addressPlaceholder: "you@gmail.com",
    note: "Requires app password",
    guideUrl: "https://myaccount.google.com/apppasswords",
  },
  {
    id: "outlook",
    name: "Outlook",
    host: "smtp.office365.com",
    port: 587,
    addressPlaceholder: "you@outlook.com",
    note: "Modern auth supported",
    guideUrl:
      "https://support.microsoft.com/en-us/account-billing/using-app-passwords-with-apps-that-don-t-support-two-step-verification-5896ed9b-4263-e681-128a-a6f2979a7944",
  },
  {
    id: "icloud",
    name: "iCloud",
    host: "smtp.mail.me.com",
    port: 587,
    addressPlaceholder: "you@icloud.com",
    note: "Requires app password",
    guideUrl: "https://support.apple.com/en-us/102654",
  },
  {
    id: "custom",
    name: "Custom",
    host: "",
    port: 0,
    addressPlaceholder: "you@example.com",
    note: "Fill in your provider's settings",
  },
];

function detectProviderId(host: string): Provider["id"] {
  const match = PROVIDERS.find((p) => p.host && p.host === host);
  return match?.id ?? "custom";
}

// ─── Step indicator ─────────────────────────────────────────────────────────

type StepState = "done" | "active" | "upcoming";

function StepBadge({
  state,
  num,
}: {
  state: StepState;
  num: number;
}) {
  if (state === "done") {
    return (
      <div className="w-[18px] h-[18px] rounded-full bg-pine text-canvas flex items-center justify-center">
        <Check size={10} strokeWidth={2.5} />
      </div>
    );
  }
  if (state === "active") {
    return (
      <div
        className="w-[18px] h-[18px] rounded-full bg-pine text-canvas flex items-center justify-center"
        style={{ fontSize: "10px", fontWeight: 600 }}
      >
        {num}
      </div>
    );
  }
  return (
    <div
      className="w-[18px] h-[18px] rounded-full border border-rule text-ink-faint flex items-center justify-center"
      style={{ fontSize: "10px", fontWeight: 600 }}
    >
      {num}
    </div>
  );
}

function Stepper({
  current,
  completedMap,
}: {
  current: number;
  completedMap: Record<number, boolean>;
}) {
  const steps = [
    { n: 1, label: "Provider" },
    { n: 2, label: "Credentials" },
    { n: 3, label: "Test" },
  ];
  return (
    <div className="flex items-center gap-2 mb-6">
      {steps.map((s, i) => {
        const state: StepState = completedMap[s.n]
          ? "done"
          : current === s.n
          ? "active"
          : "upcoming";
        return (
          <div key={s.n} className="flex items-center gap-2 flex-1">
            <div className="flex items-center gap-1.5 shrink-0">
              <StepBadge state={state} num={s.n} />
              <span
                className={`text-[11.5px] ${
                  state === "upcoming" ? "text-ink-faint" : "text-ink"
                } ${state === "active" ? "font-semibold" : "font-normal"}`}
              >
                {s.label}
              </span>
            </div>
            {i < steps.length - 1 && (
              <div className="flex-1 h-px bg-rule" />
            )}
          </div>
        );
      })}
    </div>
  );
}

// ─── Wizard ─────────────────────────────────────────────────────────────────

interface EmailSetupWizardProps {
  email: EmailConfig;
  onChange: (patch: Partial<EmailConfig>) => void;
  onTest: () => void;
  testing: boolean;
  testResult: { ok: true } | { ok: false; error: string } | null;
}

export default function EmailSetupWizard({
  email,
  onChange,
  onTest,
  testing,
  testResult,
}: EmailSetupWizardProps) {
  const [advanced, setAdvanced] = useState(false);
  const [showPass, setShowPass] = useState(false);

  const selectedId = useMemo(() => detectProviderId(email.smtp_host), [
    email.smtp_host,
  ]);
  const provider = PROVIDERS.find((p) => p.id === selectedId)!;

  // Step completion logic.
  const step1Done = email.smtp_host.trim().length > 0 && email.smtp_port > 0;
  const step2Done =
    step1Done &&
    email.smtp_user.trim().length > 0 &&
    email.smtp_password.trim().length > 0 &&
    email.recipient.trim().length > 0;
  const step3Done = step2Done && testResult?.ok === true;

  const currentStep = step3Done ? 3 : step2Done ? 3 : step1Done ? 2 : 1;

  const selectProvider = (p: Provider) => {
    if (p.id === "custom") {
      onChange({ smtp_host: "", smtp_port: 587 });
      setAdvanced(true);
    } else {
      onChange({ smtp_host: p.host, smtp_port: p.port });
    }
  };

  // Derived display for confirmation card.
  const hasPreset =
    selectedId !== "custom" && email.smtp_host && email.smtp_port > 0;

  return (
    <div>
      {/* Header */}
      <SmallCaps>Email delivery</SmallCaps>
      <Serif
        as="h3"
        size={22}
        weight={500}
        className="mt-1.5 mb-1.5 leading-[1.2]"
      >
        Email delivery
      </Serif>
      <p className="text-[13px] text-ink-muted leading-[1.5] max-w-[420px] mb-6">
        Send each newsletter to your inbox automatically.
      </p>

      {/* Stepper */}
      <Stepper
        current={currentStep}
        completedMap={{ 1: step1Done, 2: step2Done, 3: step3Done }}
      />

      {/* Step 1 — Provider */}
      <div className="mb-6">
        <p className="text-[12px] font-medium text-ink-soft mb-2.5">
          Where do you receive email?
        </p>
        <div className="grid grid-cols-2 gap-2">
          {PROVIDERS.map((p) => {
            const active = selectedId === p.id;
            return (
              <button
                key={p.id}
                onClick={() => selectProvider(p)}
                className={`
                  text-left px-3.5 py-3 rounded-lg
                  transition-colors duration-150
                  ${
                    active
                      ? "bg-sage-soft border-[1.5px] border-pine"
                      : "bg-surface border border-rule hover:border-ink-faint"
                  }
                `}
              >
                <Serif as="div" size={16} weight={500} className="mb-0.5">
                  {p.name}
                </Serif>
                <div className="text-[11px] text-ink-muted">{p.note}</div>
              </button>
            );
          })}
        </div>
      </div>

      {/* Auto-fill confirmation */}
      {hasPreset && (
        <div className="bg-surface-sunk border border-rule-soft rounded-lg px-3.5 py-3 mb-6 text-[12px] text-ink-soft leading-[1.55]">
          <div className="flex items-center gap-1.5 text-moss font-medium mb-0.5">
            <Check size={12} strokeWidth={2.5} />
            Auto-filled from {provider.name}
          </div>
          <div>
            Host:{" "}
            <span className="text-ink font-medium">{email.smtp_host}</span> ·
            Port:{" "}
            <span className="text-ink font-medium">{email.smtp_port}</span>
          </div>
        </div>
      )}

      {/* Step 2 — Credentials */}
      <div className="space-y-4 mb-4">
        <div>
          <label className="label">
            Your {provider.name === "Custom" ? "" : `${provider.name} `}address
          </label>
          <input
            type="email"
            value={email.smtp_user}
            onChange={(e) => onChange({ smtp_user: e.target.value })}
            placeholder={provider.addressPlaceholder}
            className="input"
          />
        </div>

        <div>
          <label className="label">App password</label>
          <div className="relative">
            <input
              type={showPass ? "text" : "password"}
              value={email.smtp_password}
              onChange={(e) => onChange({ smtp_password: e.target.value })}
              placeholder="16-character code"
              className="input pr-10"
            />
            <button
              type="button"
              onClick={() => setShowPass(!showPass)}
              className="absolute right-2.5 top-1/2 -translate-y-1/2 text-ink-faint hover:text-ink-soft"
            >
              {showPass ? <EyeOff size={15} /> : <Eye size={15} />}
            </button>
          </div>

          {/* Sage callout */}
          {provider.id !== "custom" && (
            <div className="mt-2 bg-sage-soft border-l-2 border-moss rounded-r-md px-3 py-2.5 text-[11.5px] text-ink-soft leading-[1.55]">
              <strong className="text-ink font-semibold">
                How to make one for {provider.name}:
              </strong>{" "}
              Visit your {provider.name} account → Security → App passwords.
              Create one labeled "Arboretum" and paste the code above.
              {provider.guideUrl && (
                <>
                  {" "}
                  <a
                    href={provider.guideUrl}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-pine underline underline-offset-2 decoration-pine/40 hover:decoration-pine whitespace-nowrap"
                  >
                    Open guide →
                  </a>
                </>
              )}
            </div>
          )}
        </div>

        <div>
          <label className="label">Send to</label>
          <input
            type="email"
            value={email.recipient}
            onChange={(e) => onChange({ recipient: e.target.value })}
            placeholder="research@lab.example"
            className="input"
          />
        </div>
      </div>

      {/* Advanced */}
      <button
        type="button"
        onClick={() => setAdvanced(!advanced)}
        className="
          text-[12px] text-ink-muted hover:text-ink-soft
          flex items-center gap-1.5 py-2 transition-colors
        "
      >
        {advanced ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
        Advanced settings
      </button>

      {advanced && (
        <div className="bg-surface-sunk rounded-lg px-3.5 py-3 grid grid-cols-[2fr_1fr] gap-3.5 mb-4">
          <div>
            <label className="label">SMTP host</label>
            <input
              type="text"
              value={email.smtp_host}
              onChange={(e) => onChange({ smtp_host: e.target.value })}
              placeholder="smtp.example.com"
              className="input"
            />
          </div>
          <div>
            <label className="label">Port</label>
            <input
              type="number"
              value={email.smtp_port || ""}
              onChange={(e) =>
                onChange({ smtp_port: Number(e.target.value) || 0 })
              }
              placeholder="587"
              className="input"
            />
          </div>
        </div>
      )}

      {/* Step 3 — Test */}
      <div className="flex items-center gap-3 pt-4 border-t border-rule-soft mt-4">
        <button
          onClick={onTest}
          disabled={testing || !email.enabled}
          className="btn-secondary text-[12.5px] py-1.5 px-3"
        >
          {testing ? (
            <Loader size={13} className="animate-spin" />
          ) : (
            <Plug size={13} strokeWidth={1.7} />
          )}
          {testing ? "Testing…" : "Test connection"}
        </button>

        {testResult?.ok === true && (
          <span className="flex items-center gap-1.5 text-[12px] text-success">
            <CheckCircle size={13} />
            Connection verified &amp; saved
          </span>
        )}
        {testResult?.ok === false && (
          <span
            className="flex items-center gap-1.5 text-[12px] text-rust max-w-[340px]"
            title={testResult.error}
          >
            <AlertCircle size={13} className="shrink-0" />
            <span className="truncate">{testResult.error}</span>
          </span>
        )}
      </div>
    </div>
  );
}
