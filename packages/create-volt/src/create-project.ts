import { cpSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { updateVoltConfigContent } from './config-updater.js';
import { escapeHtml, ProjectOptions } from './options.js';

const __dirname = fileURLToPath(new URL('.', import.meta.url));

export async function createProject(options: ProjectOptions): Promise<void> {
  const targetDir = resolve(process.cwd(), options.name);

  if (existsSync(targetDir)) {
    console.error(`  Error: Directory "${options.name}" already exists.`);
    process.exit(1);
  }

  const templateName = `${options.framework}-ts`;
  const templateDir = resolve(__dirname, 'templates', templateName);

  if (!existsSync(templateDir)) {
    console.error(`  Error: Template "${templateName}" not found.`);
    process.exit(1);
  }

  console.log(`  Creating project in ${targetDir}...`);
  console.log();

  mkdirSync(targetDir, { recursive: true });
  cpSync(templateDir, targetDir, { recursive: true });

  const pkgPath = resolve(targetDir, 'package.json');
  if (existsSync(pkgPath)) {
    const pkg = JSON.parse(readFileSync(pkgPath, 'utf-8')) as { name?: string };
    pkg.name = options.name;
    writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n');
  }

  const configPath = resolve(targetDir, 'volt.config.ts');
  if (existsSync(configPath)) {
    const config = updateVoltConfigContent(readFileSync(configPath, 'utf-8'), options.displayName);
    writeFileSync(configPath, config);
  }

  const htmlPath = resolve(targetDir, 'index.html');
  if (existsSync(htmlPath)) {
    let html = readFileSync(htmlPath, 'utf-8');
    html = html.replace(/<title>.*?<\/title>/is, `<title>${escapeHtml(options.displayName)}</title>`);
    writeFileSync(htmlPath, html);
  }

  console.log('  Done! Next steps:');
  console.log();
  console.log(`  cd ${options.name}`);
  console.log('  npm install');
  console.log('  npm run dev');
  console.log();
}
