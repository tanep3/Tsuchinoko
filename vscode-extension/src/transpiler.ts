import * as vscode from 'vscode';
import * as path from 'path';
import * as os from 'os';
import * as fs from 'fs';
import * as crypto from 'crypto';
import { execSync } from 'child_process';

function getTnkPath(): string {
    const config = vscode.workspace.getConfiguration('tsuchinoko');
    return config.get('tnkPath', 'tnk');
}

function getOutputPath(pythonFilePath: string): string {
    const outputDir = path.join(os.tmpdir(), 'tsuchinoko');
    if (!fs.existsSync(outputDir)) {
        fs.mkdirSync(outputDir, { recursive: true });
    }
    const hash = crypto.createHash('md5')
        .update(pythonFilePath)
        .digest('hex')
        .slice(0, 8);
    return path.join(outputDir, `preview_${hash}.rs`);
}

export async function transpileFile(pythonFilePath: string): Promise<string> {
    const tnkPath = getTnkPath();
    const outputPath = getOutputPath(pythonFilePath);

    try {
        execSync(`"${tnkPath}" "${pythonFilePath}" -o "${outputPath}"`, {
            encoding: 'utf-8',
            timeout: 30000,
            windowsHide: true
        });

        return fs.readFileSync(outputPath, 'utf-8');
    } catch (error: any) {
        const stderr = error.stderr || error.message;
        throw new Error(stderr);
    }
}

export async function checkFile(pythonFilePath: string): Promise<{ success: boolean; errors: string[] }> {
    const tnkPath = getTnkPath();

    try {
        execSync(`"${tnkPath}" "${pythonFilePath}" --check`, {
            encoding: 'utf-8',
            timeout: 10000,
            windowsHide: true
        });
        return { success: true, errors: [] };
    } catch (error: any) {
        const stderr = error.stderr || error.message;
        return { success: false, errors: [stderr] };
    }
}
