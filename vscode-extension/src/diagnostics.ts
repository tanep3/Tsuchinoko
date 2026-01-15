import * as vscode from 'vscode';
import { checkFile, TnkDiagnostic } from './transpiler';

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
        const diagnostics = convertDiagnostics(result.diagnostics, document);
        diagnosticCollection.set(document.uri, diagnostics);
    }
}

function convertDiagnostics(
    tnkDiags: TnkDiagnostic[],
    document: vscode.TextDocument
): vscode.Diagnostic[] {
    return tnkDiags.map(diag => {
        // Use accurate position information from diagnostic span
        const startLine = Math.max(0, diag.span.line - 1); // Convert to 0-indexed
        const startColumn = Math.max(0, diag.span.column - 1); // Convert to 0-indexed
        const endLine = Math.max(0, diag.span.end_line - 1); // Convert to 0-indexed
        const endColumn = Math.max(0, diag.span.end_column - 1); // Convert to 0-indexed

        // Ensure we don't exceed line length
        const startLineText = startLine < document.lineCount ? document.lineAt(startLine).text : '';
        const endLineText = endLine < document.lineCount ? document.lineAt(endLine).text : '';

        const safeStartColumn = Math.min(startColumn, startLineText.length);
        const safeEndColumn = Math.min(endColumn, endLineText.length);

        const range = new vscode.Range(
            new vscode.Position(startLine, safeStartColumn),
            new vscode.Position(endLine, safeEndColumn)
        );

        // Map severity
        const severity = mapSeverity(diag.severity);

        const diagnostic = new vscode.Diagnostic(range, diag.message, severity);
        diagnostic.source = 'tsuchinoko';
        diagnostic.code = diag.code;

        return diagnostic;
    });
}

function mapSeverity(severity: string): vscode.DiagnosticSeverity {
    switch (severity) {
        case 'error': return vscode.DiagnosticSeverity.Error;
        case 'warning': return vscode.DiagnosticSeverity.Warning;
        case 'info': return vscode.DiagnosticSeverity.Information;
        default: return vscode.DiagnosticSeverity.Error;
    }
}
