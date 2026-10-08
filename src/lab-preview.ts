/** Build an inert offline preview; no scripts or network requests are allowed. */
export interface PreviewSample {
  url: string;
  resources: Record<string, string>;
}
export function previewDocument(html: string, sample: PreviewSample): string {
  const doc = new DOMParser().parseFromString(html, "text/html");
  const resource = (raw: string, base = sample.url) => {
    try {
      return sample.resources[new URL(raw, base).href] || "";
    } catch {
      return "";
    }
  };
  function css(text: string, base: string): string {
    return text
      .replace(/url\(\s*(['"]?)(.*?)\1\s*\)/gi, (_all, _quote, raw) => {
        const data = /^data:[^;]+;base64,[a-zA-Z0-9+/=]+$/.test(raw)
          ? raw
          : resource(raw, base);
        return `url("${data}")`;
      })
      .replace(/@import\s+(?:url\([^)]*\)|['"][^'"]*['"])[^;]*;/gi, "");
  }
  doc
    .querySelectorAll(
      "script, iframe, frame, frameset, object, embed, base, meta, audio, video, source",
    )
    .forEach((el) => el.remove());
  doc.querySelectorAll("link").forEach((link) => {
    const data = resource(link.getAttribute("href") || "");
    if (
      link.getAttribute("rel") === "stylesheet" &&
      data.startsWith("data:text/css;base64,")
    ) {
      const style = doc.createElement("style");
      try {
        const bytes = Uint8Array.from(atob(data.split(",")[1]), (c) =>
          c.charCodeAt(0),
        );
        style.textContent = css(
          new TextDecoder().decode(bytes),
          new URL(link.getAttribute("href")!, sample.url).href,
        );
        link.replaceWith(style);
      } catch {
        link.remove();
      }
    } else link.remove();
  });
  doc.querySelectorAll("*").forEach((el) => {
    for (const attr of Array.from(el.attributes)) {
      if (
        attr.name.toLowerCase().startsWith("on") ||
        [
          "srcdoc",
          "srcset",
          "action",
          "formaction",
          "ping",
          "autofocus",
          "nonce",
          "integrity",
        ].includes(attr.name.toLowerCase())
      )
        el.removeAttribute(attr.name);
    }
    if (el.hasAttribute("style"))
      el.setAttribute("style", css(el.getAttribute("style")!, sample.url));
    // No remote requests, including SVG image resources and lazy-loaded images.
    if (el.tagName === "IMG") {
      const raw =
        el.getAttribute("data-src") ||
        el.getAttribute("data-original") ||
        el.getAttribute("src") ||
        "";
      el.setAttribute("src", resource(raw));
    } else if (el.hasAttribute("src")) el.removeAttribute("src");
    if (
      el.tagName.toLowerCase() === "image" ||
      el.tagName.toLowerCase() === "use"
    ) {
      el.removeAttribute("href");
      el.removeAttribute("xlink:href");
    }
  });
  doc.querySelectorAll("style").forEach((style) => {
    style.textContent = css(style.textContent || "", sample.url);
  });
  const policy = doc.createElement("meta");
  policy.httpEquiv = "Content-Security-Policy";
  policy.content = `default-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'none'; base-uri 'none'; form-action 'none'`;
  doc.head.prepend(policy);
  const style = doc.createElement("style");
  style.textContent =
    "[data-mw-highlight]{outline:3px solid #7168fa!important;outline-offset:2px!important}body{cursor:crosshair!important}";
  doc.head.append(style);
  return "<!doctype html>" + doc.documentElement.outerHTML;
}

export interface InspectedElement {
  selector: string;
  exact: string;
  html: string;
  css: string;
  tag: string;
  href?: string;
  linkSelector?: string;
}
export interface Inspector {
  highlight(selector: string): void;
  parent(): void;
}
/** Event handlers run in the app; the sandboxed page cannot execute scripts. */
export function installInspector(
  frame: HTMLIFrameElement,
  onInspect: (element: InspectedElement) => void,
): Inspector | null {
  const doc = frame.contentDocument;
  if (!doc) return null;
  let selected: Element | null = null;
  function clear() {
    doc!
      .querySelectorAll("[data-mw-highlight]")
      .forEach((el) => el.removeAttribute("data-mw-highlight"));
  }
  function path(element: Element): string {
    const parts: string[] = [];
    let el: Element | null = element;
    while (el && el.tagName !== "HTML") {
      const tag = el.tagName.toLowerCase();
      if (el.id) {
        parts.unshift(`${tag}#${CSS.escape(el.id)}`);
        break;
      }
      const siblings: Element[] = el.parentElement
        ? Array.from(el.parentElement.children).filter(
            (s) => s.tagName === el!.tagName,
          )
        : [el];
      parts.unshift(
        tag +
          (siblings.length > 1
            ? `:nth-of-type(${siblings.indexOf(el) + 1})`
            : ""),
      );
      el = el.parentElement;
    }
    return parts.join(" > ");
  }
  function simple(el: Element): string | null {
    const tag = el.tagName.toLowerCase();
    return el.id
      ? `${tag}#${CSS.escape(el.id)}`
      : el.classList.length
        ? tag +
          Array.from(el.classList)
            .map((c) => "." + CSS.escape(c))
            .join("")
        : null;
  }
  function suggestion(el: Element): string {
    const direct = simple(el);
    if (direct) return direct;
    if (el.tagName === "A") {
      const href = el.getAttribute("href") || "";
      const prefix = href.slice(0, href.lastIndexOf("/") + 1);
      if (prefix && prefix !== "/" && !prefix.endsWith("://"))
        return `a[href^="${prefix.replaceAll("\\", "\\\\").replaceAll('"', '\\"')}"]`;
    }
    let parent = el.parentElement;
    for (
      let depth = 0;
      parent && depth < 3;
      depth++, parent = parent.parentElement
    ) {
      const selector = simple(parent);
      if (selector) return `${selector} ${el.tagName.toLowerCase()}`;
    }
    return path(el);
  }
  function inspect(el: Element | null) {
    if (!el || el.tagName === "HTML") return;
    clear();
    selected = el;
    const exact = path(el);
    const selector = suggestion(el);
    const computed = frame.contentWindow!.getComputedStyle(el);
    const css = [
      "display",
      "color",
      "background-color",
      "font-size",
      "width",
      "height",
      "margin",
      "padding",
    ]
      .map((k) => `${k}: ${computed.getPropertyValue(k)}`)
      .join(";\n");
    const html = el.outerHTML.slice(0, 4000);
    el.setAttribute("data-mw-highlight", "");
    const link = el.closest("a[href]");
    const linkSelector = link ? suggestion(link) : undefined;
    onInspect({
      selector,
      tag: el.tagName.toLowerCase(),
      exact,
      html,
      css,
      href: link?.getAttribute("href") || undefined,
      linkSelector,
    });
  }
  doc.addEventListener(
    "click",
    (event) => {
      event.preventDefault();
      event.stopPropagation();
      inspect(event.target as Element);
    },
    true,
  );
  doc.addEventListener("keydown", (event) => {
    const key = event.key.toLowerCase();
    const shortcut =
      event.key === "Escape" ||
      ((event.ctrlKey || event.metaKey) &&
        (event.shiftKey
          ? ["f", "t", "h", "l"].includes(key)
          : ["k", "j", ",", "arrowleft"].includes(key)));
    if (!shortcut) return;
    const forwarded = new KeyboardEvent("keydown", {
      key: event.key,
      ctrlKey: event.ctrlKey,
      metaKey: event.metaKey,
      shiftKey: event.shiftKey,
      altKey: event.altKey,
      cancelable: true,
    });
    window.dispatchEvent(forwarded);
    if (forwarded.defaultPrevented) event.preventDefault();
  });
  doc.addEventListener("submit", (event) => event.preventDefault(), true);
  return {
    parent: () => inspect(selected?.parentElement || null),
    highlight: (selector) => {
      clear();
      try {
        doc
          .querySelectorAll(selector)
          .forEach((el) => el.setAttribute("data-mw-highlight", ""));
      } catch {
        /* Rust reports invalid selectors on extraction. */
      }
    },
  };
}
