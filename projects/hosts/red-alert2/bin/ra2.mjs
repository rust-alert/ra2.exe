#!/usr/bin/env node
import { pathToFileURL } from 'node:url';

function printUsage() {
    console.log(`Usage:
  ra2 launch --path <game-dir> [--edition ra2|yr]
  ra2 --version
  ra2 --help`);
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
