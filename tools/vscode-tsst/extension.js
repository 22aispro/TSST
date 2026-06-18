const vscode = require("vscode");
const path = require("path");
const fs = require("fs");

let cachedTable = null;

function activate(context) {
    const provider = vscode.languages.registerCompletionItemProvider(
        "tsst",
        {
            async provideCompletionItems(document) {
                const items = [];

                const table = await loadCompletionTable(context);
                addTableItems(items, table);

                addSymbolsFromText(items, document.getText(), "current file");

                const importedTexts = await getImportedTexts(document);

                for (const imported of importedTexts) {
                    addSymbolsFromText(items, imported.text, imported.label);
                }

                return dedupeCompletionItems(items);
            }
        },
        "!",
        ".",
        "\"",
        "_",
        "&",
        "|"
    );

    context.subscriptions.push(provider);
}

async function loadCompletionTable(context) {
    if (cachedTable) {
        return cachedTable;
    }

    const uri = vscode.Uri.joinPath(context.extensionUri, "completions.json");

    try {
        const bytes = await vscode.workspace.fs.readFile(uri);
        const text = Buffer.from(bytes).toString("utf8");
        cachedTable = JSON.parse(text);

        if (!Array.isArray(cachedTable)) {
            cachedTable = [];
        }

        return cachedTable;
    } catch {
        cachedTable = fallbackCompletions();
        return cachedTable;
    }
}

function fallbackCompletions() {
    return [
        {
            label: "main",
            kind: "snippet",
            detail: "main function",
            insertText: "pub fcn main () {\n\t$0\n}"
        },
        {
            label: "input_str",
            kind: "function",
            detail: "read terminal input as str",
            insertText: "input_str(${1:\"Prompt: \"})"
        },
        {
            label: "input_int",
            kind: "function",
            detail: "read terminal input as int",
            insertText: "input_int(${1:\"Prompt: \"})"
        },
        {
            label: "lower",
            kind: "function",
            detail: "convert string to lowercase",
            insertText: "lower(${1:value})"
        },
        {
            label: "upper",
            kind: "function",
            detail: "convert string to uppercase",
            insertText: "upper(${1:value})"
        },
        {
            label: "trim",
            kind: "function",
            detail: "trim whitespace from string",
            insertText: "trim(${1:value})"
        },
        {
            label: "contains",
            kind: "function",
            detail: "check if a string contains another string",
            insertText: "contains(${1:value}, ${2:needle})"
        }
    ];
}

function addTableItems(items, table) {
    for (const entry of table) {
        if (!entry || !entry.label) {
            continue;
        }

        const item = new vscode.CompletionItem(entry.label, kindFromString(entry.kind));

        if (entry.detail) {
            item.detail = entry.detail;
        }

        if (entry.documentation) {
            item.documentation = new vscode.MarkdownString(entry.documentation);
        }

        if (entry.insertText) {
            if (entry.kind === "snippet" || entry.insertText.includes("$")) {
                item.insertText = new vscode.SnippetString(entry.insertText);
            } else {
                item.insertText = entry.insertText;
            }
        }

        items.push(item);
    }
}

function kindFromString(kind) {
    if (kind === "keyword") {
        return vscode.CompletionItemKind.Keyword;
    }

    if (kind === "snippet") {
        return vscode.CompletionItemKind.Snippet;
    }

    if (kind === "function") {
        return vscode.CompletionItemKind.Function;
    }

    if (kind === "variable") {
        return vscode.CompletionItemKind.Variable;
    }

    if (kind === "operator") {
        return vscode.CompletionItemKind.Operator;
    }

    if (kind === "value") {
        return vscode.CompletionItemKind.Value;
    }

    if (kind === "module") {
        return vscode.CompletionItemKind.Module;
    }

    return vscode.CompletionItemKind.Text;
}

function addSymbolsFromText(items, text, sourceLabel) {
    addFunctionsFromText(items, text, sourceLabel);
    addVariablesFromText(items, text, sourceLabel);
}

function addFunctionsFromText(items, text, sourceLabel) {
    const functionRegex = /\bfcn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)\s*(?:->\s*([A-Za-z_][A-Za-z0-9_]*))?/g;
    let match;

    while ((match = functionRegex.exec(text)) !== null) {
        const name = match[1];
        const params = cleanSpaces(match[2] || "");
        const returnType = match[3] || "void";
        const signature = `${name}(${params}) -> ${returnType}`;

        const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Function);
        item.detail = `TSST function (${sourceLabel})`;
        item.documentation = new vscode.MarkdownString("```tsst\n" + signature + "\n```");
        item.insertText = new vscode.SnippetString(buildCallSnippet(name, params));
        items.push(item);

        addParamsAsVariables(items, params, sourceLabel);
    }
}

function addVariablesFromText(items, text, sourceLabel) {
    const variableRegex = /\bcre_(int|str|bool|arr|dict)\s+([A-Za-z_][A-Za-z0-9_]*)\b/g;
    let match;

    while ((match = variableRegex.exec(text)) !== null) {
        const type = match[1];
        const name = match[2];

        const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Variable);
        item.detail = `TSST ${type} variable (${sourceLabel})`;
        item.insertText = name;
        items.push(item);
    }
}

function addParamsAsVariables(items, paramsText, sourceLabel) {
    const paramRegex = /\bcre_(int|str|bool|arr|dict)\s+([A-Za-z_][A-Za-z0-9_]*)\b/g;
    let match;

    while ((match = paramRegex.exec(paramsText)) !== null) {
        const type = match[1];
        const name = match[2];

        const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Variable);
        item.detail = `TSST ${type} parameter (${sourceLabel})`;
        item.insertText = name;
        items.push(item);
    }
}

function buildCallSnippet(name, paramsText) {
    const paramRegex = /\bcre_(?:int|str|bool|arr|dict)\s+([A-Za-z_][A-Za-z0-9_]*)\b/g;
    const params = [];
    let match;

    while ((match = paramRegex.exec(paramsText)) !== null) {
        params.push(match[1]);
    }

    if (params.length === 0) {
        return `${name}($0)`;
    }

    const placeholders = params
        .map((param, index) => `\${${index + 1}:${param}}`)
        .join(", ");

    return `${name}(${placeholders})`;
}

async function getImportedTexts(document) {
    const results = [];

    if (document.uri.scheme !== "file") {
        return results;
    }

    const text = document.getText();
    const importRegex = /^\s*use\s+"([^"]+)"\s*;/gm;
    let match;
    const seen = new Set();

    while ((match = importRegex.exec(text)) !== null) {
        const importValue = match[1];
        const resolved = resolveImport(document.uri.fsPath, importValue);

        if (!resolved) {
            continue;
        }

        if (seen.has(resolved.fullPath)) {
            continue;
        }

        seen.add(resolved.fullPath);

        try {
            const bytes = await vscode.workspace.fs.readFile(vscode.Uri.file(resolved.fullPath));
            results.push({
                label: resolved.label,
                text: Buffer.from(bytes).toString("utf8")
            });
        } catch {
        }
    }

    return results;
}

function resolveImport(currentFilePath, importValue) {
    if (importValue.includes(":") && !importValue.includes("://")) {
        return resolvePackageImport(currentFilePath, importValue);
    }

    const local = resolveLocalImport(currentFilePath, importValue);

    if (fs.existsSync(local.fullPath)) {
        return local;
    }

    if (looksLikePackageSlashImport(importValue)) {
        return resolvePackageSlashImport(currentFilePath, importValue);
    }

    return local;
}

function resolveLocalImport(currentFilePath, importValue) {
    const currentDir = path.dirname(currentFilePath);
    const fullPath = path.resolve(currentDir, importValue);

    return {
        fullPath,
        label: `imported file: ${importValue}`
    };
}

function resolvePackageImport(currentFilePath, importValue) {
    const parts = importValue.split(":");

    if (parts.length !== 2) {
        return null;
    }

    const packageName = parts[0];
    const moduleName = parts[1];

    if (!packageName || !moduleName) {
        return null;
    }

    const projectRoot = findProjectRoot(path.dirname(currentFilePath));

    if (!projectRoot) {
        return null;
    }

    const modulePath = ensureTsstExtension(moduleName.split(".").join(path.sep));
    const fullPath = path.join(projectRoot, "packages", packageName, modulePath);

    return {
        fullPath,
        label: `package: ${packageName}:${moduleName}`
    };
}

function resolvePackageSlashImport(currentFilePath, importValue) {
    const projectRoot = findProjectRoot(path.dirname(currentFilePath));

    if (!projectRoot) {
        return null;
    }

    const fullPath = path.join(projectRoot, "packages", ensureTsstExtension(importValue));

    return {
        fullPath,
        label: `package: ${importValue}`
    };
}

function looksLikePackageSlashImport(importValue) {
    return !importValue.startsWith("./")
        && !importValue.startsWith("../")
        && !importValue.endsWith(".tsst")
        && importValue.includes("/");
}

function ensureTsstExtension(value) {
    if (value.endsWith(".tsst")) {
        return value;
    }

    return `${value}.tsst`;
}

function findProjectRoot(startDir) {
    let current = startDir;

    while (true) {
        const manifestPath = path.join(current, "tsst.json");

        try {
            const stat = fs.statSync(manifestPath);

            if (stat.isFile()) {
                return current;
            }
        } catch {
        }

        const parent = path.dirname(current);

        if (parent === current) {
            return null;
        }

        current = parent;
    }
}

function dedupeCompletionItems(items) {
    const seen = new Set();
    const result = [];

    for (const item of items) {
        const key = `${item.kind}:${item.label}:${item.detail || ""}`;

        if (seen.has(key)) {
            continue;
        }

        seen.add(key);
        result.push(item);
    }

    return result;
}

function cleanSpaces(value) {
    return value.replace(/\s+/g, " ").trim();
}

function deactivate() {}

module.exports = { activate, deactivate };