#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT_DIR = path.resolve(__dirname, '..');
const LOCALES_DIR = path.join(ROOT_DIR, 'src', 'lib', 'i18n', 'locales');
const SRC_DIR = path.join(ROOT_DIR, 'src');
const RUST_DIR = path.join(ROOT_DIR, 'src-tauri', 'src');
const PLURAL_CATEGORIES = new Set(['zero', 'one', 'two', 'few', 'many', 'other']);

const args = process.argv.slice(2);
const isStrict = args.includes('--strict');
const showUnused = args.includes('--unused') || args.includes('-u');
const asJson = args.includes('--json');

const c = {
  reset: '\x1b[0m',
  bold: '\x1b[1m',
  dim: '\x1b[2m',
  red: '\x1b[31m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  cyan: '\x1b[36m',
  gray: '\x1b[90m',
};

function isPluralForms(val) {
  if (val === null || typeof val !== 'object' || Array.isArray(val)) return false;
  const keys = Object.keys(val);
  return keys.includes('other') && keys.every((k) => PLURAL_CATEGORIES.has(k) && typeof val[k] === 'string');
}

function flattenObject(obj, prefix = '') {
  const result = {};
  for (const [key, val] of Object.entries(obj)) {
    const fullPath = prefix ? `${prefix}.${key}` : key;
    if (isPluralForms(val)) {
      result[fullPath] = val;
    } else if (val !== null && typeof val === 'object' && !Array.isArray(val)) {
      Object.assign(result, flattenObject(val, fullPath));
    } else {
      result[fullPath] = val;
    }
  }
  return result;
}

function messageStrings(val) {
  return typeof val === 'string' ? [val] : Object.values(val);
}

function extractPlaceholders(val) {
  const names = new Set();
  for (const str of messageStrings(val)) {
    for (const m of str.matchAll(/\{([a-zA-Z0-9_]+)\}/g)) names.add(m[1]);
  }
  return [...names].sort();
}

function missingPluralForms(localeName, val) {
  if (!isPluralForms(val)) return [];
  const required = new Intl.PluralRules(localeName).resolvedOptions().pluralCategories;
  return required.filter((category) => !(category in val));
}

function findSourceFiles(dir, extensions = ['.svelte', '.ts', '.js']) {
  let files = [];
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name !== 'node_modules' && entry.name !== 'locales') {
        files = files.concat(findSourceFiles(fullPath, extensions));
      }
    } else if (entry.isFile() && extensions.includes(path.extname(entry.name))) {
      files.push(fullPath);
    }
  }
  return files;
}

function extractKeysFromCode(files) {
  const codeKeys = new Map();
  const tCallRegex = /(?:i18n\.)?t\s*\(\s*([^,\n)]+)/g;
  const stringLiteralRegex = /['"`]([a-zA-Z0-9_]+(?:\.[a-zA-Z0-9_]+)+)['"`]/g;

  for (const file of files) {
    const content = fs.readFileSync(file, 'utf8');
    const lines = content.split('\n');
    const relPath = path.relative(ROOT_DIR, file).replace(/\\/g, '/');

    lines.forEach((line, idx) => {
      const lineNum = idx + 1;
      let match;
      while ((match = tCallRegex.exec(line)) !== null) {
        const firstArg = match[1];
        let strMatch;
        while ((strMatch = stringLiteralRegex.exec(firstArg)) !== null) {
          const key = strMatch[1];
          if (!codeKeys.has(key)) {
            codeKeys.set(key, new Set());
          }
          codeKeys.get(key).add(`${relPath}:${lineNum}`);
        }
      }
    });
  }

  return codeKeys;
}

// Keys also arrive as plain strings (`labelKey`, backend `label_key`), not only through t().
function collectKeyLiterals(files) {
  const literals = new Map();
  const prefixes = new Set();
  for (const file of files) {
    const content = fs.readFileSync(file, 'utf8');
    const relPath = path.relative(ROOT_DIR, file).replace(/\\/g, '/');
    const literalRegex = file.endsWith('.rs')
      ? /_key:\s*(?:Some\()?"([a-z][a-z0-9_]*(?:\.[a-z0-9_]+)+)"/g
      : /['"`]([a-z][a-z0-9_]*(?:\.[a-z0-9_]+)+)['"`]/g;
    for (const m of content.matchAll(literalRegex)) {
      if (!literals.has(m[1])) literals.set(m[1], new Set());
      literals.get(m[1]).add(relPath);
    }
    for (const m of content.matchAll(/['"`]([a-z][a-z0-9_]*\.[a-z0-9_.]*)(?:\$\{|['"]\s*\+)/g)) {
      prefixes.add(m[1]);
    }
  }
  return { literals, prefixes };
}

function runCheck() {
  if (!fs.existsSync(LOCALES_DIR)) {
    console.error(`Locales directory not found: ${LOCALES_DIR}`);
    process.exit(1);
  }

  const localeFiles = fs.readdirSync(LOCALES_DIR).filter(f => f.endsWith('.json'));
  if (localeFiles.length === 0) {
    console.error(`No .json locale files found in ${LOCALES_DIR}`);
    process.exit(1);
  }

  const locales = {};
  const flattenedLocales = {};
  const allKeysSet = new Set();

  for (const file of localeFiles) {
    const localeName = path.basename(file, '.json');
    const fullPath = path.join(LOCALES_DIR, file);
    try {
      const content = JSON.parse(fs.readFileSync(fullPath, 'utf8'));
      locales[localeName] = content;
      const flat = flattenObject(content);
      flattenedLocales[localeName] = flat;
      for (const k of Object.keys(flat)) {
        allKeysSet.add(k);
      }
    } catch (err) {
      console.error(`Error parsing JSON in ${file}: ${err.message}`);
      process.exit(1);
    }
  }

  const allKeys = Array.from(allKeysSet).sort();
  const namespaces = new Set(allKeys.map((k) => k.split('.')[0]));
  const sourceFiles = findSourceFiles(SRC_DIR);
  const codeKeysMap = extractKeysFromCode(sourceFiles);
  const { literals, prefixes } = collectKeyLiterals([...sourceFiles, ...findSourceFiles(RUST_DIR, ['.rs'])]);
  for (const [literal, refs] of literals) {
    if (namespaces.has(literal.split('.')[0]) && !codeKeysMap.has(literal)) {
      codeKeysMap.set(literal, refs);
    }
  }

  const missingInLocale = {};
  const emptyInLocale = {};
  const missingPluralForm = [];
  const placeholderMismatches = [];

  for (const localeName of Object.keys(locales)) {
    missingInLocale[localeName] = [];
    emptyInLocale[localeName] = [];
    const flat = flattenedLocales[localeName];

    for (const key of allKeys) {
      if (!(key in flat)) {
        missingInLocale[localeName].push(key);
        continue;
      }
      const val = flat[key];
      if (messageStrings(val).some((str) => str.trim() === '')) {
        emptyInLocale[localeName].push(key);
      }
      for (const category of missingPluralForms(localeName, val)) {
        missingPluralForm.push(`${localeName}: ${key} (${category})`);
      }
    }
  }

  for (const key of allKeys) {
    const placeholdersPerLocale = {};
    for (const [localeName, flat] of Object.entries(flattenedLocales)) {
      if (key in flat) {
        placeholdersPerLocale[localeName] = extractPlaceholders(flat[key]);
      }
    }

    const entries = Object.entries(placeholdersPerLocale);
    if (entries.length > 1) {
      const [firstLocale, firstVars] = entries[0];
      for (let i = 1; i < entries.length; i++) {
        const [otherLocale, otherVars] = entries[i];
        const firstStr = firstVars.join(',');
        const otherStr = otherVars.join(',');
        if (firstStr !== otherStr) {
          placeholderMismatches.push({
            key,
            localeA: firstLocale,
            varsA: firstVars,
            localeB: otherLocale,
            varsB: otherVars,
          });
          break;
        }
      }
    }
  }

  const missingInTranslations = [];
  for (const [codeKey, refs] of codeKeysMap.entries()) {
    const isNamespacePrefix = allKeys.some((k) => k.startsWith(`${codeKey}.`));
    if (!allKeysSet.has(codeKey) && !isNamespacePrefix) {
      missingInTranslations.push({ key: codeKey, refs: Array.from(refs) });
    }
  }

  const unusedKeys = [];
  for (const key of allKeys) {
    const builtFromPrefix = [...prefixes].some((prefix) => key.startsWith(prefix));
    if (!codeKeysMap.has(key) && !builtFromPrefix) {
      unusedKeys.push(key);
    }
  }

  let hasErrors = false;
  for (const keys of Object.values(missingInLocale)) {
    if (keys.length > 0) hasErrors = true;
  }
  for (const keys of Object.values(emptyInLocale)) {
    if (keys.length > 0) hasErrors = true;
  }
  if (missingInTranslations.length > 0 || missingPluralForm.length > 0) {
    hasErrors = true;
  }

  if (asJson) {
    console.log(JSON.stringify({
      totalKeys: allKeys.length,
      locales: Object.keys(locales),
      missingInLocale,
      emptyInLocale,
      missingPluralForm,
      placeholderMismatches,
      missingInTranslations,
      unusedKeys,
      hasErrors,
    }, null, 2));
    process.exit(hasErrors ? 1 : 0);
  }

  console.log(`\ni18n status: ${allKeys.length} keys across ${Object.keys(locales).join(', ')}`);

  for (const [localeName, flat] of Object.entries(flattenedLocales)) {
    const count = Object.keys(flat).length;
    const missing = missingInLocale[localeName].length;
    const empty = emptyInLocale[localeName].length;

    let status = `${c.green}ok (${count}/${allKeys.length})${c.reset}`;
    if (missing > 0 || empty > 0) {
      status = `${c.red}missing ${missing}, empty ${empty}${c.reset}`;
    }
    console.log(`  • ${localeName.padEnd(4)} : ${status}`);
  }

  if (missingInTranslations.length > 0) {
    console.log(`\n${c.red}Keys missing in translation files (${missingInTranslations.length}):${c.reset}`);
    for (const item of missingInTranslations) {
      console.log(`  ${c.red}-${c.reset} ${item.key}`);
      for (const ref of item.refs.slice(0, 3)) {
        console.log(`    ${c.gray}-> ${ref}${c.reset}`);
      }
    }
  }

  for (const [localeName, keys] of Object.entries(missingInLocale)) {
    if (keys.length > 0) {
      console.log(`\n${c.red}Keys missing in ${localeName} (${keys.length}):${c.reset}`);
      for (const k of keys.slice(0, 15)) {
        console.log(`  - ${k}`);
      }
      if (keys.length > 15) {
        console.log(`  ... +${keys.length - 15} more`);
      }
    }
  }

  for (const [localeName, keys] of Object.entries(emptyInLocale)) {
    if (keys.length > 0) {
      console.log(`\n${c.yellow}Empty keys in ${localeName} (${keys.length}):${c.reset}`);
      for (const k of keys) {
        console.log(`  - ${k}`);
      }
    }
  }

  if (missingPluralForm.length > 0) {
    console.log(`\n${c.red}Missing plural forms (${missingPluralForm.length}):${c.reset}`);
    for (const item of missingPluralForm) {
      console.log(`  - ${item}`);
    }
  }

  if (placeholderMismatches.length > 0) {
    console.log(`\n${c.yellow}Placeholder mismatches (${placeholderMismatches.length}):${c.reset}`);
    for (const item of placeholderMismatches) {
      console.log(`  ${item.key}: ${item.localeA}={${item.varsA.join(',')}} vs ${item.localeB}={${item.varsB.join(',')}}`);
    }
  }

  if (unusedKeys.length > 0 && showUnused) {
    console.log(`\n${c.gray}Keys not referenced in code (${unusedKeys.length}):${c.reset}`);
    for (const k of unusedKeys) {
      console.log(`  ${c.gray}• ${k}${c.reset}`);
    }
  }

  if (hasErrors) {
    console.log(`\n${c.red}i18n check failed.${c.reset}\n`);
    process.exit(1);
  } else if (isStrict && (unusedKeys.length > 0 || placeholderMismatches.length > 0)) {
    console.log(`\n${c.yellow}i18n strict check failed on warnings.${c.reset}\n`);
    process.exit(1);
  } else {
    console.log(`\n${c.green}i18n check passed.${c.reset}\n`);
    process.exit(0);
  }
}

runCheck();
