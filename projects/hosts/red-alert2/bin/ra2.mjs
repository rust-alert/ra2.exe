#!/usr/bin/env node
import { pathToFileURL } from 'node:url';

function printUsage() {
    console.log(`Usage:
  ra2 launch --path <game-dir> [--edition ra2|yr]
  ra2 extract --path <game-dir> --out <dir> [--edition ra2|yr] [--palette name.pal] [--decode-shp] [--] <name>...
  ra2 unpack --path <game-dir> --out <dir> [--edition ra2|yr]
  ra2 --version
  ra2 --help

Examples:
  ra2 extract --path "C:/Games/RA2" --out ./out --decode-shp -- sdtp.shp title.pcx
  ra2 unpack --path "C:/Games/RA2" --out ./unpacked`);
}

function parsePathEditionOut(args, command) {
    let gamePath = null;
    let out = null;
    let edition;
    const rest = [];

    for (let i = 0; i < args.length; i += 1) {
        const a = args[i];
        if (a === '--path') {
            gamePath = args[i + 1];
            if (!gamePath) {
                throw new Error(`${command}: --path requires a directory`);
            }
            i += 1;
            continue;
        }
        if (a === '--out') {
            out = args[i + 1];
            if (!out) {
                throw new Error(`${command}: --out requires a directory`);
            }
            i += 1;
            continue;
        }
        if (a === '--edition') {
            edition = args[i + 1];
            if (!edition) {
                throw new Error(`${command}: --edition requires a value`);
            }
            i += 1;
            continue;
        }
        rest.push(a);
    }

    if (!gamePath) {
        throw new Error(`${command}: --path is required`);
    }
    if (!out) {
        throw new Error(`${command}: --out is required`);
    }
    return { path: gamePath, out, edition, rest };
}

function parseExtractArgs(args) {
    let gamePath = null;
    let out = null;
    let edition;
    let palette;
    let decodeShp = false;
    const names = [];
    let afterSep = false;

    for (let i = 0; i < args.length; i += 1) {
        const a = args[i];
        if (!afterSep && a === '--') {
            afterSep = true;
            continue;
        }
        if (!afterSep && a === '--path') {
            gamePath = args[i + 1];
            if (!gamePath) {
                throw new Error('extract: --path requires a directory');
            }
            i += 1;
            continue;
        }
        if (!afterSep && a === '--out') {
            out = args[i + 1];
            if (!out) {
                throw new Error('extract: --out requires a directory');
            }
            i += 1;
            continue;
        }
        if (!afterSep && a === '--edition') {
            edition = args[i + 1];
            if (!edition) {
                throw new Error('extract: --edition requires a value');
            }
            i += 1;
            continue;
        }
        if (!afterSep && a === '--palette') {
            palette = args[i + 1];
            if (!palette) {
                throw new Error('extract: --palette requires a value');
            }
            i += 1;
            continue;
        }
        if (!afterSep && a === '--decode-shp') {
            decodeShp = true;
            continue;
        }
        if (!afterSep && a.startsWith('-')) {
            throw new Error(`extract: unknown argument ${a}`);
        }
        names.push(a);
    }

    if (!gamePath) {
        throw new Error('extract: --path is required');
    }
    if (!out) {
        throw new Error('extract: --out is required');
    }
    if (names.length === 0) {
        throw new Error('extract: at least one logical name is required');
    }

    return { path: gamePath, out, edition, palette, decodeShp, names };
}

async function main() {
    const args = process.argv.slice(2);
    if (args.length === 0 || args.includes('--help') || args.includes('-h')) {
        printUsage();
        process.exit(args.length === 0 ? 1 : 0);
    }

    if (args[0] === '--version' || args[0] === '-V') {
        const { version } = await import('../dist/native.js');
        console.log(version());
        return;
    }

    if (args[0] === 'launch') {
        let gamePath = null;
        let edition;
        for (let i = 1; i < args.length; i += 1) {
            const a = args[i];
            if (a === '--path') {
                gamePath = args[i + 1];
                if (!gamePath) {
                    console.error('ra2 launch: --path requires a directory');
                    process.exit(1);
                }
                i += 1;
            } else if (a === '--edition') {
                edition = args[i + 1];
                if (!edition) {
                    console.error('ra2 launch: --edition requires a value');
                    process.exit(1);
                }
                i += 1;
            } else {
                console.error(`ra2 launch: unknown argument ${a}`);
                printUsage();
                process.exit(1);
            }
        }
        if (!gamePath) {
            console.error('ra2 launch: --path is required');
            printUsage();
            process.exit(1);
        }
        const { launch } = await import('../dist/native.js');
        launch({ path: gamePath, edition });
        return;
    }

    if (args[0] === 'extract') {
        let opts;
        try {
            opts = parseExtractArgs(args.slice(1));
        } catch (err) {
            console.error(err instanceof Error ? err.message : String(err));
            printUsage();
            process.exit(1);
        }
        const { extract } = await import('../dist/native.js');
        const result = extract({
            path: opts.path,
            out: opts.out,
            edition: opts.edition,
            palette: opts.palette,
            decodeShp: opts.decodeShp,
            names: opts.names,
        });
        console.log(
            `edition=${result.edition} root_mix=${result.mountedRoot} nested=${result.mountedNested} written=${result.written.length} missing=${result.missing.length}`,
        );
        for (const f of result.written) {
            const frames = f.shpFrames != null ? ` frames=${f.shpFrames}` : '';
            console.log(`OK ${f.name} -> ${f.path} (${f.bytes} bytes, ${f.origin})${frames}`);
        }
        for (const name of result.missing) {
            console.log(`MISSING ${name}`);
        }
        if (result.missing.length > 0) {
            process.exit(2);
        }
        return;
    }

    if (args[0] === 'unpack') {
        let opts;
        try {
            opts = parsePathEditionOut(args.slice(1), 'unpack');
            if (opts.rest.length > 0) {
                throw new Error(`unpack: unexpected argument ${opts.rest[0]} (full dump needs no names; use extract for named files)`);
            }
        } catch (err) {
            console.error(err instanceof Error ? err.message : String(err));
            printUsage();
            process.exit(1);
        }
        const { unpack } = await import('../dist/native.js');
        const result = unpack({
            path: opts.path,
            out: opts.out,
            edition: opts.edition,
        });
        console.log(
            `edition=${result.edition} root_mix=${result.mountedRoot} nested=${result.mountedNested} archives=${result.archives} files=${result.filesWritten} bytes=${result.bytesWritten} out=${result.outDir}`,
        );
        return;
    }

    console.error(`ra2: unknown command ${args[0]}`);
    printUsage();
    process.exit(1);
}

const isDirect = process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href;
if (isDirect || process.argv[1]?.endsWith('ra2.mjs') || process.argv[1]?.endsWith('ra2')) {
    main().catch((err) => {
        console.error(err instanceof Error ? err.message : String(err));
        process.exit(1);
    });
}
