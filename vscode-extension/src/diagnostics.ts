import * as vscode from 'vscode';
import { checkFile } from './transpiler';

export const diagnosticCollection = vscode.languages.createDiagnosticCollection('tsuchinoko');

export async function updateDiagnostics(document: vscode.TextDocument): Promise<void> {
    if (document.languageId !== 'python') {
        return;
    }

    const config = vscode.workspace.getConfiguration('tsuchinoko');
    const delay = config.get('checkDelay', 500);

    // Debounce
    await new Promise(resolve => setTimeout(resolve, delay));

    const result = await checkFile(document.fileName);

    if (result.success) {
        diagnosticCollection.set(document.uri, []);
    } else {
        const diagnostics = parseErrors(result.errors, document);
        diagnosticCollection.set(document.uri, diagnostics);
    }
}

function parseErrors(errors: string[], document: vscode.TextDocument): vscode.Diagnostic[] {
    const diagnostics: vscode.Diagnostic[] = [];

    for (const error of errors) {
        // Try to parse line number from error message
        // Common patterns: "line 15:", "at line 15", "L15:"
        const lineMatch = error.match(/(?:line|L|:)\s*(\d+)/i);
        let line = 0;

        if (lineMatch) {
            line = Math.max(0, parseInt(lineMatch[1], 10) - 1);
        }

        const range = new vscode.Range(
            new vscode.Position(line, 0),
            new vscode.Position(line, document.lineAt(Math.min(line, document.lineCount - 1)).text.length)
        );

        const diagnostic = new vscode.Diagnostic(
            range,
            error,
            vscode.DiagnosticSeverity.Error
        );
        diagnostic.source = 'tsuchinoko';
        diagnostic.code = 'TSUCHINOKO001';

        diagnostics.push(diagnostic);
    }

    return diagnostics;
}
