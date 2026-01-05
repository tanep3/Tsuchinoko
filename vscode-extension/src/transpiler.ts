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
