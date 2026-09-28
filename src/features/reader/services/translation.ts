type MyMemoryResponse = {
  responseData?: { translatedText?: string };
  responseStatus?: number;
  responseDetails?: string;
};

function truncateUtf8(text: string, maxBytes: number) {
  const encoder = new TextEncoder();
  if (encoder.encode(text).length <= maxBytes) return text;
  let result = "";
  for (const character of text) {
    if (encoder.encode(result + character).length > maxBytes) break;
    result += character;
  }
  return result;
}

function decodeHtml(text: string) {
  const element = document.createElement("textarea");
  element.innerHTML = text;
  return element.value;
}

export async function translateText(text: string) {
  const source = /[\u3400-\u9fff]/u.test(text) ? "zh-CN" : "en";
  const target = source === "en" ? "zh-CN" : "en";
  const query = truncateUtf8(text.trim(), 500);
  const url = new URL("https://api.mymemory.translated.net/get");
  url.searchParams.set("q", query);
  url.searchParams.set("langpair", `${source}|${target}`);
  const response = await fetch(url, { headers: { Accept: "application/json" } });
  if (!response.ok) throw new Error(`翻译服务返回 ${response.status}`);
  const payload = await response.json() as MyMemoryResponse;
  const translated = payload.responseData?.translatedText;
  if (!translated || (payload.responseStatus && payload.responseStatus >= 400)) {
    throw new Error(payload.responseDetails || "翻译服务没有返回结果");
  }
  return {
    sourceLanguage: source,
    targetLanguage: target,
    sourceText: query,
    translatedText: decodeHtml(translated),
    truncated: query !== text.trim(),
  };
}
