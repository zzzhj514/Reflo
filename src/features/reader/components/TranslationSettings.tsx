import { useEffect, useState } from "react";
import type { TranslationPreferences, TranslationProvider, TranslationTargetLanguage } from "../../../shared/contracts/translation";
import { getTranslationSettings, saveTranslationSettings } from "../../../shared/ipc/translation";

const PRESETS: Record<Exclude<TranslationProvider, "custom">, Pick<TranslationPreferences, "baseUrl" | "model">> = {
  openai: { baseUrl: "https://api.openai.com/v1", model: "gpt-4o-mini" },
  deepseek: { baseUrl: "https://api.deepseek.com", model: "deepseek-chat" },
};

const DEFAULTS: TranslationPreferences = {
  provider: "deepseek",
  baseUrl: PRESETS.deepseek.baseUrl,
  model: PRESETS.deepseek.model,
  targetLanguage: "auto",
};

const TARGETS: Array<[TranslationTargetLanguage, string]> = [
  ["auto", "自动：中译英，其他译中"], ["zh-CN", "简体中文"], ["en", "英文"],
  ["ja", "日文"], ["ko", "韩文"], ["fr", "法文"], ["de", "德文"], ["es", "西班牙文"],
];

export function TranslationSettings() {
  const [preferences, setPreferences] = useState<TranslationPreferences>(DEFAULTS);
  const [apiKey, setApiKey] = useState("");
  const [keyConfigured, setKeyConfigured] = useState(false);
  const [configuredProviders, setConfiguredProviders] = useState<TranslationProvider[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    getTranslationSettings().then((status) => {
      if (!active) return;
      setPreferences(status.preferences);
      setKeyConfigured(status.keyConfigured);
      setConfiguredProviders(status.configuredProviders);
    }).catch((cause) => { if (active) setError(String(cause)); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, []);

  function chooseProvider(provider: TranslationProvider) {
    const preset = provider === "custom" ? null : PRESETS[provider];
    setPreferences((current) => ({
      ...current,
      provider,
      baseUrl: preset?.baseUrl ?? current.baseUrl,
      model: preset?.model ?? current.model,
    }));
    setApiKey("");
    setKeyConfigured(configuredProviders.includes(provider));
    setMessage(null);
  }

  async function save() {
    if (saving || (!keyConfigured && !apiKey.trim())) return;
    setSaving(true);
    setMessage(null);
    setError(null);
    try {
      const result = await saveTranslationSettings(preferences, apiKey || undefined);
      setPreferences(result.preferences);
      setKeyConfigured(result.keyConfigured);
      setConfiguredProviders(result.configuredProviders);
      setApiKey("");
      setMessage("翻译设置已保存。");
    } catch (cause) { setError(String(cause)); }
    finally { setSaving(false); }
  }

  if (loading) return <p className="muted" role="status">正在读取翻译设置…</p>;

  return (
    <div className="translation-settings">
      <label>服务商<select value={preferences.provider} onChange={(event) => chooseProvider(event.target.value as TranslationProvider)}>
        <option value="deepseek">DeepSeek</option>
        <option value="openai">OpenAI</option>
        <option value="custom">自定义兼容 API</option>
      </select></label>
      <label>API Base URL<input type="url" value={preferences.baseUrl}
        onChange={(event) => setPreferences({ ...preferences, baseUrl: event.target.value })} /></label>
      <label>模型<input value={preferences.model}
        onChange={(event) => setPreferences({ ...preferences, model: event.target.value })} /></label>
      <label>目标语言<select value={preferences.targetLanguage}
        onChange={(event) => setPreferences({ ...preferences, targetLanguage: event.target.value as TranslationTargetLanguage })}>
        {TARGETS.map(([value, label]) => <option key={value} value={value}>{label}</option>)}
      </select></label>
      <label>API Key<input type="password" autoComplete="new-password" value={apiKey}
        placeholder={keyConfigured ? "已保存在 macOS 钥匙串；留空保持不变" : "输入 API Key"}
        onChange={(event) => setApiKey(event.target.value)} /></label>
      <p className="muted">请求使用 OpenAI-compatible Chat Completions 格式。API Key 只存入 macOS 钥匙串。</p>
      {message && <p className="settings-success" role="status">{message}</p>}
      {error && <p className="form-error" role="alert">{error}</p>}
      <button className="primary-button" disabled={saving || (!keyConfigured && !apiKey.trim())}
        onClick={() => void save()}>{saving ? "保存中…" : "保存翻译设置"}</button>
    </div>
  );
}
