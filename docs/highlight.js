document.addEventListener("DOMContentLoaded", () => {
  const blocks = document.querySelectorAll("code.language-tsst");

  blocks.forEach((block) => {
    const raw = block.textContent;
    block.innerHTML = highlightTSST(raw);
  });
});

function escapeHTML(value) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

function highlightTSST(code) {
  let html = escapeHTML(code);

  const placeholders = [];

  function stash(match, htmlValue) {
    const key = `__TSST_PLACEHOLDER_${placeholders.length}__`;
    placeholders.push([key, htmlValue]);
    return key;
  }

  html = html.replace(/\/\/.*$/gm, (match) =>
    stash(match, `<span class="tok-comment">${match}</span>`)
  );

  html = html.replace(/"([^"\\]|\\.)*"/g, (match) =>
    stash(match, `<span class="tok-string">${match}</span>`)
  );

  html = html.replace(
    /\b(pub|fcn|return|if|else|while|for|in|break|continue|use)\b/g,
    `<span class="tok-keyword">$1</span>`
  );

  html = html.replace(
    /\b(cre_int|cre_str|cre_bool|cre_arr|cre_dict)\b/g,
    `<span class="tok-type">$1</span>`
  );

  html = html.replace(
    /\b(true|false)\b/g,
    `<span class="tok-bool">$1</span>`
  );

  html = html.replace(
    /\b(cons|push|set)!(?=\()/g,
    `<span class="tok-macro">$1!</span>`
  );

  html = html.replace(
    /\b(len)(?=\()/g,
    `<span class="tok-builtin">$1</span>`
  );

  html = html.replace(
    /\b\d+\b/g,
    `<span class="tok-number">$&</span>`
  );

  html = html.replace(
    /(==|!=|&lt;=|&gt;=|-&gt;|=|&lt;|&gt;|\+|-|\*|\/)/g,
    `<span class="tok-op">$1</span>`
  );

  for (const [key, value] of placeholders) {
    html = html.replaceAll(key, value);
  }

  return html;
}
