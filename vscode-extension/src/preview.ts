import * as vscode from 'vscode';

let currentPanel: vscode.WebviewPanel | undefined;

export function showPreview(
    context: vscode.ExtensionContext,
    rustCode: string,
    fileName: string
) {
    const columnToShowIn = vscode.ViewColumn.Beside;

    if (currentPanel) {
        // If panel exists, update content
        currentPanel.webview.html = getWebviewContent(rustCode, fileName);
        currentPanel.reveal(columnToShowIn);
    } else {
        // Create new panel
        currentPanel = vscode.window.createWebviewPanel(
            'tsuchinokoPreview',
            `Rust: ${fileName}`,
            columnToShowIn,
            {
                enableScripts: false,
                retainContextWhenHidden: true
            }
        );

        currentPanel.webview.html = getWebviewContent(rustCode, fileName);

        // Reset when closed
        currentPanel.onDidDispose(
            () => {
                currentPanel = undefined;
            },
            null,
            context.subscriptions
        );
    }
}

function getWebviewContent(rustCode: string, fileName: string): string {
    const escapedCode = escapeHtml(rustCode);

    return `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Rust Preview: ${fileName}</title>
    <style>
        body {
            font-family: 'Cascadia Code', 'Fira Code', Consolas, Monaco, monospace;
            padding: 16px;
            margin: 0;
            background-color: var(--vscode-editor-background);
            color: var(--vscode-editor-foreground);
        }
        pre {
            margin: 0;
            white-space: pre-wrap;
            word-wrap: break-word;
            font-size: 13px;
            line-height: 1.5;
        }
        .header {
            font-size: 14px;
            font-weight: bold;
            margin-bottom: 12px;
            padding-bottom: 8px;
            border-bottom: 1px solid var(--vscode-panel-border);
            color: var(--vscode-descriptionForeground);
        }
        .rust-icon {
            margin-right: 6px;
        }
    </style>
</head>
<body>
    <div class="header">
        <span class="rust-icon">🦀</span>
        Transpiled from ${fileName}
    </div>
    <pre>${escapedCode}</pre>
</body>
</html>`;
}

function escapeHtml(text: string): string {
    return text
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#039;');
}
