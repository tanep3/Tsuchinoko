import * as vscode from 'vscode';
import * as path from 'path';
import * as os from 'os';
import * as fs from 'fs';
import { transpileFile, checkFile } from './transpiler';
import { showPreview } from './preview';
import { updateDiagnostics, diagnosticCollection } from './diagnostics';

let outputChannel: vscode.OutputChannel;

function cleanupTempFiles() {
    const outputDir = path.join(os.tmpdir(), 'tsuchinoko');
    if (fs.existsSync(outputDir)) {
        try {
            fs.rmSync(outputDir, { recursive: true, force: true });
        } catch (e) {
            // Ignore cleanup errors
        }
    }
}

export function activate(context: vscode.ExtensionContext) {
    // Cleanup previous temp files (crash recovery)
    cleanupTempFiles();

    // Create output channel for logging
    outputChannel = vscode.window.createOutputChannel('Tsuchinoko');
    outputChannel.appendLine('Tsuchinoko extension activated');

    // Register preview command
    const previewCommand = vscode.commands.registerCommand('tsuchinoko.preview', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor || editor.document.languageId !== 'python') {
            vscode.window.showWarningMessage('Please open a Python file first');
            return;
        }

        const filePath = editor.document.fileName;
        outputChannel.appendLine(`Transpiling: ${filePath}`);

        try {
            const rustCode = await transpileFile(filePath);
            showPreview(context, rustCode, path.basename(filePath));
        } catch (error: any) {
            vscode.window.showErrorMessage(`Transpilation failed: ${error.message}`);
            outputChannel.appendLine(`Error: ${error.message}`);
        }
    });

    // Register transpile command
    const transpileCommand = vscode.commands.registerCommand('tsuchinoko.transpile', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor || editor.document.languageId !== 'python') {
            vscode.window.showWarningMessage('Please open a Python file first');
            return;
        }

        const filePath = editor.document.fileName;
        const outputPath = filePath.replace(/\.py$/, '.rs');

        try {
            const rustCode = await transpileFile(filePath);
            fs.writeFileSync(outputPath, rustCode);
            vscode.window.showInformationMessage(`Transpiled to: ${outputPath}`);
        } catch (error: any) {
            vscode.window.showErrorMessage(`Transpilation failed: ${error.message}`);
        }
    });

    // Auto-check on save
    const config = vscode.workspace.getConfiguration('tsuchinoko');
    if (config.get('autoCheck', true)) {
        const onSave = vscode.workspace.onDidSaveTextDocument((document) => {
            if (document.languageId === 'python') {
                updateDiagnostics(document);
            }
        });
        context.subscriptions.push(onSave);
    }

    // Create status bar button
    const statusBarItem = vscode.window.createStatusBarItem(
        vscode.StatusBarAlignment.Right,
        100
    );
    statusBarItem.text = "$(rocket) Rust Preview";
    statusBarItem.tooltip = "Tsuchinoko: Show Rust Preview (Alt+R)";
    statusBarItem.command = "tsuchinoko.preview";

    // Show/hide status bar based on active editor
    const updateStatusBar = () => {
        const editor = vscode.window.activeTextEditor;
        if (editor && editor.document.languageId === 'python') {
            statusBarItem.show();
        } else {
            statusBarItem.hide();
        }
    };

    updateStatusBar();
    context.subscriptions.push(
        vscode.window.onDidChangeActiveTextEditor(updateStatusBar)
    );

    // Register disposables
    context.subscriptions.push(previewCommand);
    context.subscriptions.push(transpileCommand);
    context.subscriptions.push(diagnosticCollection);
    context.subscriptions.push(outputChannel);
    context.subscriptions.push(statusBarItem);
}

export function deactivate() {
    cleanupTempFiles();
}
