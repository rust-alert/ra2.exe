#!/usr/bin/env node
import { pathToFileURL } from 'node:url';

function printUsage() {
    console.log(`Usage:
  ra2 emulate --path <game-dir> [--edition ra2|yr] [--screen skirmish|main|campaign|...]
  ra2 extract --path <game-dir> --out <dir> [--edition ra2|yr] [--theater temperate|snow|...] [--palette name.pal] [--decode-shp] [--decode-csf] [--] <name>...
  ra2 unpack --path <game-dir> --out <dir> [--edition ra2|yr] [--names-file <txt>] [--decode-csf]
  ra2 diagnose-maps --path <game-dir> [--edition ra2|yr] [--limit N] [--json]
  ra2 diagnose-mobile-vxl --path <game-dir> (--stem <stem> | --type <TYPE>) [--edition ra2|yr]
      [--body-facing N] [--turret-facing N] [--hva-frame N]
      [--sweep-body | --sweep-turret | --sweep-hva] [--hva-frames N] [--json]
  ra2 --version
  ra2 --help

Screens (emulate --screen):
  splash (default), main, single, campaign, skirmish, choose_map, options

Examples:
  ra2 emulate --path "C:/Games/RA2" --edition ra2 --screen skirmish
  ra2 extract --path "C:/Games/RA2" --out ./out --decode-shp -- sdtp.shp title.pcx
  ra2 extract --path "C:/Games/RA2" --out ./out --edition ra2 --decode-shp --palette isotem.pal -- tibtre01.tem
  ra2 extract --path "C:/Games/RA2" --out ./out --decode-csf -- ra2.csf
  ra2 unpack --path "C:/Games/RA2" --out ./unpacked
  ra2 unpack --path "C:/Games/RA2" --out ./unpacked --names-file ./extra_names.txt --decode-csf
  ra2 diagnose-maps --path "C:/Games/RA2" --edition ra2
  ra2 diagnose-maps --path "C:/Games/RA2" --edition ra2 --limit 5
  ra2 diagnose-maps --path "C:/Games/RA2" --edition ra2 --json
  ra2 diagnose-mobile-vxl --path "C:/Games/RA2" --edition ra2 --stem mtnk
  ra2 diagnose-mobile-vxl --path "C:/Games/RA2" --edition ra2 --type MTNK --sweep-turret --json
  ra2 diagnose-mobile-vxl --path "C:/Games/RA2" --edition ra2 --stem mtnk --sweep-hva --hva-frames 3`);
}

function parsePathEditionOut(args, command) {
    let gamePath = null;
    let out = null;
    let edition;
    let namesFile;
    let decodeCsf = false;
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
        if (a === '--names-file') {
            namesFile = args[i + 1];
            if (!namesFile) {
                throw new Error(`${command}: --names-file requires a path`);
            }
            i += 1;
            continue;
        }
        if (a === '--decode-csf') {
            decodeCsf = true;
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
    return { path: gamePath, out, edition, namesFile, decodeCsf, rest };
}

function parseExtractArgs(args) {
    let gamePath = null;
    let out = null;
    let edition;
    let palette;
    let theater;
    let decodeShp = false;
    let decodeCsf = false;
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
        if (!afterSep && a === '--theater') {
            theater = args[i + 1];
            if (!theater) {
                throw new Error('extract: --theater requires a value');
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
        if (!afterSep && a === '--decode-csf') {
            decodeCsf = true;
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

    return { path: gamePath, out, edition, palette, theater, decodeShp, decodeCsf, names };
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

    if (args[0] === 'emulate') {
        let gamePath = null;
        let edition;
        let screen;
        for (let i = 1; i < args.length; i += 1) {
            const a = args[i];
            if (a === '--path') {
                gamePath = args[i + 1];
                if (!gamePath) {
                    console.error('ra2 emulate: --path requires a directory');
                    process.exit(1);
                }
                i += 1;
            } else if (a === '--edition') {
                edition = args[i + 1];
                if (!edition) {
                    console.error('ra2 emulate: --edition requires a value');
                    process.exit(1);
                }
                i += 1;
            } else if (a === '--screen') {
                screen = args[i + 1];
                if (!screen) {
                    console.error('ra2 emulate: --screen requires a value');
                    process.exit(1);
                }
                i += 1;
            } else {
                console.error(`ra2 emulate: unknown argument ${a}`);
                printUsage();
                process.exit(1);
            }
        }
        if (!gamePath) {
            console.error('ra2 emulate: --path is required');
            printUsage();
            process.exit(1);
        }
        const { emulate } = await import('../dist/native.js');
        emulate({ path: gamePath, edition, screen });
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
            theater: opts.theater,
            decodeShp: opts.decodeShp,
            decodeCsf: opts.decodeCsf,
            names: opts.names,
        });
        console.log(
            `edition=${result.edition} root_mix=${result.mountedRoot} nested=${result.mountedNested} written=${result.written.length} missing=${result.missing.length}`,
        );
        for (const f of result.written) {
            const size = f.shpWidth != null && f.shpHeight != null ? ` size=${f.shpWidth}x${f.shpHeight}` : '';
            const frames = f.shpFrames != null ? ` frames=${f.shpFrames}` : '';
            const csf = f.csfEntries != null ? ` csf=${f.csfEntries}` : '';
            console.log(`OK ${f.name} -> ${f.path} (${f.bytes} bytes, ${f.origin})${size}${frames}${csf}`);
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
            namesFile: opts.namesFile,
            decodeCsf: opts.decodeCsf,
        });
        console.log(
            `edition=${result.edition} root_mix=${result.mountedRoot} nested=${result.mountedNested} archives=${result.archives} files=${result.filesWritten} named=${result.namedWritten} unnamed=${result.unnamedWritten} names=${result.nameTableSize} bytes=${result.bytesWritten} out=${result.outDir}`,
        );
        return;
    }

    if (args[0] === 'diagnose-maps') {
        let gamePath = null;
        let edition;
        let limit;
        let asJson = false;
        for (let i = 1; i < args.length; i += 1) {
            const a = args[i];
            if (a === '--path') {
                gamePath = args[i + 1];
                if (!gamePath) {
                    console.error('ra2 diagnose-maps: --path requires a directory');
                    process.exit(1);
                }
                i += 1;
            } else if (a === '--edition') {
                edition = args[i + 1];
                if (!edition) {
                    console.error('ra2 diagnose-maps: --edition requires a value');
                    process.exit(1);
                }
                i += 1;
            } else if (a === '--limit') {
                const raw = args[i + 1];
                if (!raw) {
                    console.error('ra2 diagnose-maps: --limit requires a number');
                    process.exit(1);
                }
                limit = Number.parseInt(raw, 10);
                if (!Number.isFinite(limit) || limit < 0) {
                    console.error('ra2 diagnose-maps: --limit must be a non-negative integer');
                    process.exit(1);
                }
                i += 1;
            } else if (a === '--json') {
                asJson = true;
            } else {
                console.error(`ra2 diagnose-maps: unknown argument ${a}`);
                printUsage();
                process.exit(1);
            }
        }
        if (!gamePath) {
            console.error('ra2 diagnose-maps: --path is required');
            printUsage();
            process.exit(1);
        }
        const { diagnoseMaps } = await import('../dist/native.js');
        const report = diagnoseMaps({ path: gamePath, edition, limit });
        if (asJson) {
            console.log(JSON.stringify(report, null, 2));
            return;
        }
        console.log(
            `edition=${report.edition} source=${report.source} candidates=${report.candidateCount} success=${report.success} reject=${report.reject} missing=${report.missing}`,
        );
        for (const row of report.maps) {
            const gaps = [...row.blockingGaps, ...row.stubGaps, ...row.deferredGaps, ...row.otherGaps];
            const gapText = gaps.length > 0 ? ` gaps=${gaps.join('|')}` : '';
            const err =
                row.parseError != null
                    ? ` parse_error=${row.parseError}`
                    : row.prepareError != null
                      ? ` prepare_error=${row.prepareError}`
                      : '';
            console.log(`${row.triState}\t${row.fileName}\tparse=${row.parseOk}\tprepare=${row.prepareOk}${err}${gapText}`);
        }
        return;
    }

    if (args[0] === 'diagnose-mobile-vxl') {
        let gamePath = null;
        let edition;
        let stem;
        let typeId;
        let bodyFacing;
        let turretFacing;
        let hvaFrame;
        let sweepBody = false;
        let sweepTurret = false;
        let sweepHva = false;
        let hvaFrameCount;
        let asJson = false;
        for (let i = 1; i < args.length; i += 1) {
            const a = args[i];
            if (a === '--path') {
                gamePath = args[i + 1];
                if (!gamePath) {
                    console.error('ra2 diagnose-mobile-vxl: --path requires a directory');
                    process.exit(1);
                }
                i += 1;
            } else if (a === '--edition') {
                edition = args[i + 1];
                if (!edition) {
                    console.error('ra2 diagnose-mobile-vxl: --edition requires a value');
                    process.exit(1);
                }
                i += 1;
            } else if (a === '--stem') {
                stem = args[i + 1];
                if (!stem) {
                    console.error('ra2 diagnose-mobile-vxl: --stem requires a value');
                    process.exit(1);
                }
                i += 1;
            } else if (a === '--type') {
                typeId = args[i + 1];
                if (!typeId) {
                    console.error('ra2 diagnose-mobile-vxl: --type requires a value');
                    process.exit(1);
                }
                i += 1;
            } else if (a === '--body-facing') {
                const raw = args[i + 1];
                if (!raw) {
                    console.error('ra2 diagnose-mobile-vxl: --body-facing requires a number');
                    process.exit(1);
                }
                bodyFacing = Number.parseInt(raw, 10);
                if (!Number.isFinite(bodyFacing) || bodyFacing < 0 || bodyFacing > 255) {
                    console.error('ra2 diagnose-mobile-vxl: --body-facing must be 0..255');
                    process.exit(1);
                }
                i += 1;
            } else if (a === '--turret-facing') {
                const raw = args[i + 1];
                if (!raw) {
                    console.error('ra2 diagnose-mobile-vxl: --turret-facing requires a number');
                    process.exit(1);
                }
                turretFacing = Number.parseInt(raw, 10);
                if (!Number.isFinite(turretFacing) || turretFacing < 0 || turretFacing > 255) {
                    console.error('ra2 diagnose-mobile-vxl: --turret-facing must be 0..255');
                    process.exit(1);
                }
                i += 1;
            } else if (a === '--hva-frame') {
                const raw = args[i + 1];
                if (!raw) {
                    console.error('ra2 diagnose-mobile-vxl: --hva-frame requires a number');
                    process.exit(1);
                }
                hvaFrame = Number.parseInt(raw, 10);
                if (!Number.isFinite(hvaFrame) || hvaFrame < 0) {
                    console.error('ra2 diagnose-mobile-vxl: --hva-frame must be a non-negative integer');
                    process.exit(1);
                }
                i += 1;
            } else if (a === '--hva-frames') {
                const raw = args[i + 1];
                if (!raw) {
                    console.error('ra2 diagnose-mobile-vxl: --hva-frames requires a number');
                    process.exit(1);
                }
                hvaFrameCount = Number.parseInt(raw, 10);
                if (!Number.isFinite(hvaFrameCount) || hvaFrameCount < 1) {
                    console.error('ra2 diagnose-mobile-vxl: --hva-frames must be a positive integer');
                    process.exit(1);
                }
                i += 1;
            } else if (a === '--sweep-body') {
                sweepBody = true;
            } else if (a === '--sweep-turret') {
                sweepTurret = true;
            } else if (a === '--sweep-hva') {
                sweepHva = true;
            } else if (a === '--json') {
                asJson = true;
            } else {
                console.error(`ra2 diagnose-mobile-vxl: unknown argument ${a}`);
                printUsage();
                process.exit(1);
            }
        }
        if (!gamePath) {
            console.error('ra2 diagnose-mobile-vxl: --path is required');
            printUsage();
            process.exit(1);
        }
        if (!stem && !typeId) {
            console.error('ra2 diagnose-mobile-vxl: --stem or --type is required');
            printUsage();
            process.exit(1);
        }
        const sweepCount = Number(sweepBody) + Number(sweepTurret) + Number(sweepHva);
        if (sweepCount > 1) {
            console.error('ra2 diagnose-mobile-vxl: --sweep-body, --sweep-turret, and --sweep-hva are mutually exclusive');
            process.exit(1);
        }
        if (hvaFrameCount != null && !sweepHva) {
            console.error('ra2 diagnose-mobile-vxl: --hva-frames requires --sweep-hva');
            process.exit(1);
        }
        const { diagnoseMobileVxl } = await import('../dist/native.js');
        const result = diagnoseMobileVxl({
            path: gamePath,
            edition,
            stem,
            typeId,
            bodyFacing,
            turretFacing,
            hvaFrame,
            sweepBody,
            sweepTurret,
            sweepHva,
            hvaFrameCount,
        });
        if (asJson) {
            console.log(JSON.stringify(result, null, 2));
            return;
        }
        const typeText = result.typeId != null ? ` type=${result.typeId}` : '';
        console.log(
            `edition=${result.edition} stem=${result.stem}${typeText} root_mix=${result.mountedRoot} nested=${result.mountedNested} reports=${result.reports.length}`,
        );
        for (const report of result.reports) {
            console.log(
                `--- body_facing=${report.bodyFacing} turret_facing=${report.turretFacing} hva_frame=${report.hvaFrame}`,
            );
            for (const note of report.notes) {
                console.log(`note\t${note}`);
            }
            console.log(
                'role\tvxl\thva\tvxl_hit\thva_hit\tfacing\tw\th\toffset_x\toffset_y\tcell_ox\tcell_oy\torigin_px\torigin_py',
            );
            for (const layer of report.layers) {
                const dash = (v) => (v == null ? '-' : String(v));
                console.log(
                    [
                        layer.role,
                        layer.vxlName || '-',
                        layer.hvaName || '-',
                        layer.vxlHit,
                        layer.hvaHit,
                        dash(layer.facing),
                        dash(layer.width),
                        dash(layer.height),
                        dash(layer.offsetX),
                        dash(layer.offsetY),
                        dash(layer.cellOffsetX),
                        dash(layer.cellOffsetY),
                        dash(layer.originPx),
                        dash(layer.originPy),
                    ].join('\t'),
                );
            }
        }
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
