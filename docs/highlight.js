document.addEventListener("DOMContentLoaded", () => {
  const tsstBlocks = document.querySelectorAll("code.language-tsst");
  const jsonBlocks = document.querySelectorAll("code.language-json");
  const shellBlocks = document.querySelectorAll("code.language-powershell");

  tsstBlocks.forEach((block) => {
    const raw = block.textContent;
    block.innerHTML = highlightTSST(raw);
  });

  jsonBlocks.forEach((block) => {
    const raw = block.textContent;
    block.innerHTML = highlightJSON(raw);
  });

  shellBlocks.forEach((block) => {
    const raw = block.textContent;
    block.innerHTML = highlightShell(raw);
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

function highlightJSON(code) {
  let html = "";
  let i = 0;

  while (i < code.length) {
    const char = code[i];

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

      let nextIndex = i;

      while (nextIndex < code.length && /\s/.test(code[nextIndex])) {
        nextIndex++;
      }

      if (code[nextIndex] === ":") {
        html += span("tok-json-key", value);
      } else {
        html += span("tok-json-string", value);
      }

      continue;
    }

    if ("{}[]:,".includes(char)) {
      html += span("tok-json-punc", char);
      i++;
      continue;
    }

    html += escapeHTML(char);
    i++;
  }

  return html;
}

function highlightShell(code) {
  let html = "";
  let i = 0;

  while (i < code.length) {
    const char = code[i];

    if (char === "-") {
      let value = "";

      while (i < code.length && !/\s/.test(code[i])) {
        value += code[i];
        i++;
      }

      html += span("tok-shell-flag", value);
      continue;
    }

    if (/[A-Za-z.\\]/.test(char)) {
      let value = "";

      while (i < code.length && !/\s/.test(code[i])) {
        value += code[i];
        i++;
      }

      if (isShellCommand(value)) {
        html += span("tok-shell-command", value);
      } else {
        html += escapeHTML(value);
      }

      continue;
    }

    html += escapeHTML(char);
    i++;
  }

  return html;
}

function isShellCommand(value) {
  const command = value.toLowerCase();

  return [
    "git",
    "cd",
    "cargo",
    "tsst",
    "code",
    "vsce",
    "npm",
    ".\\target\\release\\tsst.exe",
  ].includes(command);
}
