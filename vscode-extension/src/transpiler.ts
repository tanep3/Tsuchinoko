import * as vscode from 'vscode';
import * as path from 'path';
import * as os from 'os';
import * as fs from 'fs';
import * as crypto from 'crypto';
import { execSync } from 'child_process';

// Type definitions for diagnostic JSON output
export interface TnkDiagnostic {
    severity: 'error' | 'warning' | 'info';
    code: string;
    message: string;
    span: {
        file: string;
        line: number;
        column: number;
        end_line: number;
        end_column: number;
    };
    phase: string;
}

export interface CheckResult {
    success: boolean;
    diagnostics: TnkDiagnostic[];
}

function getTnkPath(): string {
    const config = vscode.workspace.getConfiguration('tsuchinoko');
    return config.get('tnkPath', 'tnk');
}

function getOutputDir(): string {
    const outputDir = path.join(os.tmpdir(), 'tsuchinoko');
    if (!fs.existsSync(outputDir)) {
        fs.mkdirSync(outputDir, { recursive: true });
    }
    return outputDir;
}

function getHash(filePath: string): string {
    return crypto.createHash('md5')
        .update(filePath)
        .digest('hex')
        .slice(0, 8);
}

function hasImports(pythonFilePath: string): boolean {
    const content = fs.readFileSync(pythonFilePath, 'utf-8');
    // Check for import statements (but exclude typing imports which are handled)
    const lines = content.split('\n');
    for (const line of lines) {
        const trimmed = line.trim();
        // Skip typing imports (supported)
        if (trimmed.startsWith('from typing import')) continue;
        // Check for other imports
        if (trimmed.startsWith('import ') || trimmed.startsWith('from ')) {
            return true;
        }
    }
    return false;
}

export async function transpileFile(pythonFilePath: string): Promise<string> {
    const tnkPath = getTnkPath();
    const outputDir = getOutputDir();
    const hash = getHash(pythonFilePath);

    // Check if file has imports (excluding typing)
    if (hasImports(pythonFilePath)) {
        // Use --project for files with imports
        return transpileWithProject(pythonFilePath, tnkPath, outputDir, hash);
    } else {
        // Use simple -o for files without imports
        return transpileSimple(pythonFilePath, tnkPath, outputDir, hash);
    }
}

async function transpileSimple(
    pythonFilePath: string,
    tnkPath: string,
    outputDir: string,
    hash: string
): Promise<string> {
    const outputPath = path.join(outputDir, `preview_${hash}.rs`);

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

async function transpileWithProject(
    pythonFilePath: string,
    tnkPath: string,
    outputDir: string,
    hash: string
): Promise<string> {
    const projectDir = path.join(outputDir, `project_${hash}`);
    const mainRsPath = path.join(projectDir, 'src', 'main.rs');

    // Clean up old project if exists
    if (fs.existsSync(projectDir)) {
        fs.rmSync(projectDir, { recursive: true, force: true });
    }

    try {
        // Generate Cargo project with --project
        execSync(`"${tnkPath}" "${pythonFilePath}" --project "${projectDir}"`, {
            encoding: 'utf-8',
            timeout: 60000,  // Longer timeout for project generation
            windowsHide: true
        });

        // Read the generated main.rs
        if (fs.existsSync(mainRsPath)) {
            const rustCode = fs.readFileSync(mainRsPath, 'utf-8');

            // Add header comment indicating this is a project build
            const header = `// 📦 Generated with --project (imports detected)
// Full project at: ${projectDir}
// Run: cd ${projectDir} && cargo run

`;
            return header + rustCode;
        } else {
            throw new Error(`main.rs not found at ${mainRsPath}`);
        }
    } catch (error: any) {
        const stderr = error.stderr || error.message;
        throw new Error(stderr);
    }
}

export async function checkFile(pythonFilePath: string): Promise<CheckResult> {
    const tnkPath = getTnkPath();

    try {
        // Try with --diag-json first
        const result = execSync(`"${tnkPath}" "${pythonFilePath}" --check --diag-json`, {
            encoding: 'utf-8',
            timeout: 10000,
            windowsHide: true
        });

        // If successful, no diagnostics
        return { success: true, diagnostics: [] };
    } catch (error: any) {
        const stderr = error.stderr || '';

        // Try to parse JSON diagnostics from stderr
        try {
            const jsonMatch = stderr.match(/\{[\s\S]*"diagnostics"[\s\S]*\}/m);
            if (jsonMatch) {
                const diagJson = JSON.parse(jsonMatch[0]);
                return {
                    success: false,
                    diagnostics: diagJson.diagnostics || []
                };
            }
        } catch (parseError) {
            // JSON parsing failed, fall back to legacy text-based error
        }

        // Fallback: create a simple diagnostic from error message
        const fallbackDiag: TnkDiagnostic = {
            severity: 'error',
            code: 'TSUCHINOKO-ERROR',
            message: stderr || error.message || 'Unknown error',
            span: {
                file: pythonFilePath,
                line: 1,
                column: 1,
                end_line: 1,
                end_column: 2
            },
            phase: 'unknown'
        };

        return {
            success: false,
            diagnostics: [fallbackDiag]
        };
    }
}
