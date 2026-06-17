const vscode = require("vscode");
const path = require("path");

function activate(context) {
    const provider = vscode.languages.registerCompletionItemProvider(
        "tsst",
        {
            async provideCompletionItems(document) {
                const items = [];

                addStaticItems(items);

                const currentText = document.getText();
                addSymbolsFromText(items, currentText, "current file");

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
        "_"
    );

    context.subscriptions.push(provider);
}

function addStaticItems(items) {
    function keyword(label, detail) {
        const item = new vscode.CompletionItem(label, vscode.CompletionItemKind.Keyword);
        item.detail = detail;
        items.push(item);
    }

    function snippet(label, insertText, detail) {
        const item = new vscode.CompletionItem(label, vscode.CompletionItemKind.Snippet);
        item.insertText = new vscode.SnippetString(insertText);
        item.detail = detail;
        items.push(item);
    }

    function fn(label, insertText, detail) {
        const item = new vscode.CompletionItem(label, vscode.CompletionItemKind.Function);
        item.insertText = new vscode.SnippetString(insertText);
        item.detail = detail;
        items.push(item);
    }

    keyword("pub", "public function keyword");
    keyword("fcn", "function keyword");
    keyword("return", "return from function");
    keyword("if", "if statement");
    keyword("else", "else statement");
    keyword("while", "while loop");
    keyword("for", "for loop");
    keyword("in", "for-each keyword");
    keyword("break", "break out of loop");
    keyword("continue", "continue loop");
    keyword("use", "import another TSST file or package");

    keyword("cre_int", "create int variable");
    keyword("cre_str", "create string variable");
    keyword("cre_bool", "create bool variable");
    keyword("cre_arr", "create array variable");
    keyword("cre_dict", "create dictionary variable");

    fn("len", "len(${1:value})", "get length of string, array, or dictionary");

    snippet("main", "pub fcn main () {\n\t$0\n}", "main function");

    snippet(
        "fcn",
        "fcn ${1:name} (${2}) -> ${3:int} {\n\treturn ${4:0};\n}",
        "function with return type"
    );

    snippet(
        "void fcn",
        "fcn ${1:name} (${2}) {\n\t$0\n}",
        "function with no return"
    );

    snippet("if", "if ${1:condition} {\n\t$0\n}", "if statement");

    snippet(
        "if else",
        "if ${1:condition} {\n\t$2\n} else {\n\t$0\n}",
        "if else statement"
    );

    snippet("while", "while ${1:condition} {\n\t$0\n}", "while loop");

    snippet(
        "for",
        "for (cre_int ${1:i} = 0; ${1:i} < ${2:10}; ${1:i} = ${1:i} + 1) {\n\t$0\n}",
        "classic for loop"
    );

    snippet(
        "foreach",
        "for (${1:cre_int} ${2:item} in ${3:items}) {\n\t$0\n}",
        "for-each loop"
    );

    snippet("arr", "cre_arr ${1:nums} = [${2:1, 2, 3}];", "array variable");

    snippet(
        "dict",
        "cre_dict ${1:user} = {\n\t\"${2:name}\": \"${3:John Doe}\",\n\t\"${4:age}\": ${5:37},\n};",
        "dictionary variable"
    );

    snippet("cons", "cons!(${1:value});", "print to console");
    snippet("push", "push!(${1:arr}, ${2:value});", "push value into array");
    snippet("set", "set!(${1:dict}, \"${2:key}\", ${3:value});", "set dictionary value");

    snippet("use file", "use \"${1:file.tsst}\";", "import local file");
    snippet("use package", "use \"${1:ui}:${2:ui}\";", "import package file");
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

    while ((match = importRegex.exec(text)) !== null) {
        const importValue = match[1];

        const resolved = resolveImport(document.uri.fsPath, importValue);

        if (!resolved) {
            continue;
        }

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

    return resolveLocalImport(currentFilePath, importValue);
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

    const modulePath = ensureTsstExtension(moduleName.replaceAll(".", path.sep));
    const fullPath = path.join(projectRoot, "packages", packageName, modulePath);

    return {
        fullPath,
        label: `package: ${packageName}:${moduleName}`
    };
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
            const stat = require("fs").statSync(manifestPath);

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