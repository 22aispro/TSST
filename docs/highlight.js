document.addEventListener("DOMContentLoaded", () => {
  const tsstBlocks = document.querySelectorAll("code.language-tsst");

  tsstBlocks.forEach((block) => {
    const raw = block.textContent;
    block.innerHTML = highlightTSST(raw);
  });
});

const KEYWORDS = new Set([
  "pub",
  "fcn",
  "return",
  "if",
  "else",
  "while",
  "for",
  "in",
  "break",
  "continue",
  "use",
]);

const TYPES = new Set([
  "cre_int",
  "cre_str",
  "cre_bool",
  "cre_arr",
  "cre_dict",
]);

const BOOLEANS = new Set([
  "true",
  "false",
]);

const BUILTINS = new Set([
  "len",
]);

const MACROS = new Set([
  "cons",
  "push",
  "set",
]);

function escapeHTML(value) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

function span(className, value) {
  return `<span class="${className}">${escapeHTML(value)}</span>`;
}

function isLetter(value) {
  return /[A-Za-z_]/.test(value);
}

function isDigit(value) {
  return /[0-9]/.test(value);
}

function isIdentPart(value) {
  return /[A-Za-z0-9_]/.test(value);
}

function highlightTSST(code) {
  let html = "";
  let i = 0;

  while (i < code.length) {
    const char = code[i];
    const next = code[i + 1] || "";

    if (char === "/" && next === "/") {
      let value = "";

      while (i < code.length && code[i] !== "\n") {
        value += code[i];
        i++;
      }

      html += span("tok-comment", value);
      continue;
    }

    if (char === '"') {
      let value = char;
      i++;

      while (i < code.length) {
        value += code[i];

        if (code[i] === '"' && code[i - 1] !== "\\") {
          i++;
          break;
        }

        i++;
      }

      html += span("tok-string", value);
      continue;
    }

    if (isDigit(char)) {
      let value = "";

      while (i < code.length && isDigit(code[i])) {
        value += code[i];
        i++;
      }

      html += span("tok-number", value);
      continue;
    }

    if (isLetter(char)) {
      let value = "";

      while (i < code.length && isIdentPart(code[i])) {
        value += code[i];
        i++;
      }

      if (code[i] === "!" && MACROS.has(value)) {
        html += span("tok-macro", value + "!");
        i++;
        continue;
      }

      if (KEYWORDS.has(value)) {
        html += span("tok-keyword", value);
        continue;
      }

      if (TYPES.has(value)) {
        html += span("tok-type", value);
        continue;
      }

      if (BOOLEANS.has(value)) {
        html += span("tok-bool", value);
        continue;
      }

      if (BUILTINS.has(value)) {
        html += span("tok-builtin", value);
        continue;
      }

      html += escapeHTML(value);
      continue;
    }

    const twoChar = char + next;

    if (["==", "!=", "<=", ">=", "->"].includes(twoChar)) {
      html += span("tok-op", twoChar);
      i += 2;
      continue;
    }

    if (["=", "<", ">", "+", "-", "*", "/"].includes(char)) {
      html += span("tok-op", char);
      i++;
      continue;
    }

    html += escapeHTML(char);
    i++;
  }

  return html;
}
