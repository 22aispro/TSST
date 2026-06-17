const vscode = require("vscode");

function activate(context) {
    const provider = vscode.languages.registerCompletionItemProvider(
        "tsst",
        {
            provideCompletionItems(document, position) {
                const items = [];

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

                snippet(
                    "if",
                    "if ${1:condition} {\n\t$0\n}",
                    "if statement"
                );

                snippet(
                    "if else",
                    "if ${1:condition} {\n\t$2\n} else {\n\t$0\n}",
                    "if else statement"
                );

                snippet(
                    "while",
                    "while ${1:condition} {\n\t$0\n}",
                    "while loop"
                );

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

                snippet(
                    "arr",
                    "cre_arr ${1:nums} = [${2:1, 2, 3}];",
                    "array variable"
                );

                snippet(
                    "dict",
                    "cre_dict ${1:user} = {\n\t\"${2:name}\": \"${3:Re}\",\n};",
                    "dictionary variable"
                );

                snippet(
                    "cons",
                    "cons!(${1:value});",
                    "print to console"
                );

                snippet(
                    "push",
                    "push!(${1:arr}, ${2:value});",
                    "push value into array"
                );

                snippet(
                    "set",
                    "set!(${1:dict}, \"${2:key}\", ${3:value});",
                    "set dictionary value"
                );

                snippet(
                    "use",
                    "use \"${1:file.tsst}\";",
                    "import another TSST file"
                );

                return items;
            }
        },
        "!",
        ".",
        "\"",
        "_"
    );

    context.subscriptions.push(provider);
}

function deactivate() {}

module.exports = {
    activate,
    deactivate
};