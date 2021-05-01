import { program } from 'commander';

import * as path from 'path';
import * as fs from 'fs-extra';

import * as walkSync from 'walk-sync';
import HtmlMiner from './Miners/HtmlMiner';
import CsvMiner from './Miners/CsvMiner';

program.version('0.0.1');

function getFolderPaths(folderPath, outputPath) {
  let sourceFolderPath = folderPath;
  if (!path.isAbsolute(folderPath)) {
    sourceFolderPath = path.join(process.cwd(), folderPath);
  }

  let outputFolderPath = outputPath;
  if (outputFolderPath && !path.isAbsolute(outputFolderPath)) {
    outputFolderPath = path.join(process.cwd(), outputPath);
  } else {
    outputFolderPath = path.join(process.cwd(), '_index');
  }
  return { sourceFolderPath, outputFolderPath };
}

program
  .command('html <folderPath> [outputPath]')
  .action((folderPath, outputPath) => {
    const { sourceFolderPath, outputFolderPath } = getFolderPaths(folderPath, outputPath);

    const miner = new HtmlMiner(outputFolderPath);

    const paths = walkSync(sourceFolderPath, { globs: ['**/*.html'] });
    paths.forEach((p) => {
      miner.indexHtmlDoc(p, fs.readFileSync(path.join(sourceFolderPath, p), { encoding: 'utf-8' }));
    });

    miner.dump();
  });

program
  .command('csv <folderPath> [outputPath]')
  .action((folderPath, outputPath) => {
    const { sourceFolderPath, outputFolderPath } = getFolderPaths(folderPath, outputPath);
    console.log(outputFolderPath);

    const miner = new CsvMiner(outputFolderPath);

    const paths = walkSync(sourceFolderPath, { globs: ['**/*.csv'] });
    paths.forEach((p) => {
      miner.indexCsvDoc(p, fs.readFileSync(path.join(sourceFolderPath, p), { encoding: 'utf-8' }));
    });

    miner.dump();
  });

program.parse(process.argv);
