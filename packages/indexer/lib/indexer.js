"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const commander_1 = require("commander");
const path = require("path");
const fs = require("fs-extra");
const walkSync = require("walk-sync");
const HtmlMiner_1 = require("./Miners/HtmlMiner");
const CsvMiner_1 = require("./Miners/CsvMiner");
commander_1.program.version('0.0.1');
function getFolderPaths(folderPath, outputPath) {
    let sourceFolderPath = folderPath;
    if (!path.isAbsolute(folderPath)) {
        sourceFolderPath = path.join(process.cwd(), folderPath);
    }
    let outputFolderPath = outputPath;
    if (outputFolderPath && !path.isAbsolute(outputFolderPath)) {
        outputFolderPath = path.join(process.cwd(), outputPath);
    }
    else {
        outputFolderPath = path.join(process.cwd(), '_index');
    }
    return { sourceFolderPath, outputFolderPath };
}
commander_1.program
    .command('html <folderPath> [outputPath]')
    .action((folderPath, outputPath) => {
    const { sourceFolderPath, outputFolderPath } = getFolderPaths(folderPath, outputPath);
    const miner = new HtmlMiner_1.default(outputFolderPath);
    const paths = walkSync(sourceFolderPath, { globs: ['**/*.html'] });
    paths.forEach((p) => {
        miner.indexHtmlDoc(p, fs.readFileSync(path.join(sourceFolderPath, p), { encoding: 'utf-8' }));
    });
    miner.dump();
});
commander_1.program
    .command('csv <folderPath> [outputPath]')
    .action((folderPath, outputPath) => {
    const { sourceFolderPath, outputFolderPath } = getFolderPaths(folderPath, outputPath);
    console.log(outputFolderPath);
    const miner = new CsvMiner_1.default(outputFolderPath);
    const paths = walkSync(sourceFolderPath, { globs: ['**/*.csv'] });
    paths.forEach((p) => {
        miner.indexCsvDoc(p, fs.readFileSync(path.join(sourceFolderPath, p), { encoding: 'utf-8' }));
    });
    miner.dump();
});
commander_1.program.parse(process.argv);
//# sourceMappingURL=indexer.js.map