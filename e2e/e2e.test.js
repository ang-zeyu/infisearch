
const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

jest.setTimeout(3000000);

const INPUT_SELECTOR = '#morsels-search';

async function clearInput() {
  await page.click(INPUT_SELECTOR, { clickCount: 3 });
  await page.keyboard.press('Backspace');
  const numChildren = await page.evaluate(() => {
    return document.getElementById('target-mode-el').childNodes.length;
  });
  expect(numChildren).toBe(0);
}

async function typePhraseOrAnd(phrase, with_positions) {
  if (with_positions) {
    console.log(`Typing phrase '${phrase}'`);
    await page.type(INPUT_SELECTOR, `"${phrase}"`);
    const inputVal = await page.evaluate(() => document.getElementById('morsels-search').value);
    expect(inputVal).toBe(`"${phrase}"`);
  } else {
    const query = phrase.split(/\s+/g).join(' AND ');
    console.log(`Typing AND '${query}'`);
    await page.type(INPUT_SELECTOR, query);
    const inputVal = await page.evaluate(() => document.getElementById('morsels-search').value);
    expect(inputVal).toBe(query);
  }
}

async function typeText(text) {
  console.log(`Typing text '${text}'`);
  await page.type(INPUT_SELECTOR, text);
  const inputVal = await page.evaluate(() => document.getElementById('morsels-search').value);
  expect(inputVal).toBe(text);
}

async function waitNoResults() {
  try {
    await page.waitForSelector('.morsels-no-results', { timeout: 10000 });
  } catch (ex) {
    const output = await page.evaluate(() => document.getElementById('target-mode-el').innerHTML);
    console.error('waitNoResults failed, output in target:', output);
    console.error('input element text:');
    const inputElText = await page.evaluate(() => document.getElementById('morsels-search').value);
    console.error(inputElText);
    throw ex;
  }
}

async function assertSingle(text) {
  try {
    await page.waitForSelector('.morsels-list-item', { timeout: 60000 });

    const result = await page.evaluate(() => {
      const queryResult = document.getElementsByClassName('morsels-list-item');
      return { text: queryResult.length && queryResult[0].textContent, resultCount: queryResult.length };
    });

    expect(result.resultCount).toBe(1);
    expect(result.text.toLowerCase().includes(text.toLowerCase())).toBe(true);
  } catch (ex) {
    const output = await page.evaluate(() => {
      return {
        html: document.getElementById('target-mode-el').innerHTML,
        text: document.getElementById('target-mode-el').textContent,
      };
    });
    console.error('assertSingle failed, html in target:', output.html);
    console.error('assertSingle failed, text in target:', output.text);
    throw ex;
  }
}

async function assertMultiple(texts, count) {
  try {
    await page.waitForSelector('.morsels-list-item', { timeout: 60000 });

    const result = await page.evaluate(() => {
      const queryResult = document.getElementsByClassName('morsels-list-item');
      return {
        texts: Array.from(queryResult).map((el) => el.textContent),
        resultCount: queryResult.length,
      };
    });

    expect(result.resultCount).toBe(count);
    texts.forEach((text) => {
      expect(
        result.texts.some(
          (resultText) => resultText.toLowerCase().includes(text.toLowerCase()),
        ),
      ).toBe(true);
    });
  } catch (ex) {
    const output = await page.evaluate(() => {
      return {
        html: document.getElementById('target-mode-el').innerHTML,
        text: document.getElementById('target-mode-el').textContent,
      };
    });
    console.error('assertMultiple failed, html in target:', output.html);
    console.error('assertMultiple failed, text in target:', output.text);
    throw ex;
  }
}

async function reloadPage() {
  await jestPuppeteer.resetPage();
  await jestPuppeteer.resetBrowser();

  page
    .on('console', message =>
      console.log(`${message.type()} ${message.text()}`))
    .on('error', (ex) => console.error('Unexpected (1): ' + ex))
    .on('pageerror', ({ message }) => console.error('Unexpected (2): ' + message));

  const url = 'http://localhost:8080?mode=target'
    + '&url=http%3A%2F%2Flocalhost%3A8080%2Fe2e%2Foutput%2F'
    + '&sourceFilesUrl=http%3A%2F%2Flocalhost%3A8080%2Fe2e%2Finput%2F'
    + '&resultsPerPage=100';

  await page.goto(
    url,
    { waitUntil: ['domcontentloaded', 'networkidle0'], timeout: 180000 },
  );
  await expect(page.title()).resolves.toMatch('Morsels');
}

function runIndexer(command) {
  execSync(command, {
    env: { RUST_BACKTRACE: 1, ...process.env },
    stdio: 'inherit',
  });
}

beforeAll(async () => {
  // Wait for webpack-dev-server to complete before running indexer
  await reloadPage();
});

const testSuite = async (configFile, with_positions = true) => {
  runIndexer(`cargo run -p morsels_indexer -- ./e2e/input ./e2e/output -c ${configFile}`);

  console.log('Ran full indexer run');

  await reloadPage();

  // ------------------------------------------------------
  // Various basic tests on docid=0
  await typeText('npm AND run AND dev AND installmdbook');
  await assertSingle('use the npm run dev script');

  if (with_positions) {
    await clearInput();
    await typeText('"npm run dev" AND (installmdbook 8080)');
    await assertSingle('use the npm run dev script');
  }

  await clearInput();
  await typeText('npm AND run AND dev AND nonexistentterm');
  await waitNoResults();

  await clearInput();
  await typeText('npm AND NOT run AND NOT packages');
  await waitNoResults();

  await clearInput();
  await typeText('npm AND run AND setup');
  await assertSingle('npm run setup');

  if (with_positions) {
    await clearInput();
    await typeText('body:"Once you have you test files"');
    await assertSingle('once you have you test files');
  
    await clearInput();
    await typeText('title:"Once you have you test files"');
    await waitNoResults();
  
    await clearInput();
    await typeText('title:"Developing - Morsels Documentation"');
    await assertSingle('developing - morsels documentation');
  
    await clearInput();
    await typeText('heading:"Developing - Morsels Documentation"');
    await waitNoResults();
  }
  // ------------------------------------------------------

  // ------------------------------------------------------
  // Simple phrase query test on another docid
  await clearInput();
  await typePhraseOrAnd('forenote on mobile device detection', with_positions);
  await assertSingle('forenote on mobile device detection');
  // ------------------------------------------------------

  // ------------------------------------------------------
  // Spelling correction tests
  await clearInput();
  await typeText('fornote');
  await assertMultiple([
    'forenote on stop words',
    'forenote on mobile device detection',
  ], 2);
  // ------------------------------------------------------

  // ------------------------------------------------------
  // Automatic term expansion / prefix search tests
  await clearInput();
  await typeText('foreno');
  await assertMultiple([
    'forenote on stop words',
    'forenote on mobile device detection',
  ], 2);

  await clearInput();
  await typeText('detec');
  await assertMultiple([
    'detecting deleted, changed, or added',
    'detecting such terms',
    'detected as per the earlier section',
  ], 3);
  // ------------------------------------------------------

  // ------------------------------------------------------
  // JsonLoader tests
  await clearInput();
  await typePhraseOrAnd('Lorem Ipsum is simply dummy text', with_positions);
  await assertSingle('lorem ipsum is simply dummy text');

  await clearInput();
  await typePhraseOrAnd('test many json 2', with_positions);
  await assertSingle('test many json 2');
  // ------------------------------------------------------

  // ------------------------------------------------------
  // CsvLoader tests
  // For now, the only with_positions test also uses source files to generate result previews,
  // and csvs aren't supported with this.
  if (with_positions) {
    await clearInput();
    await typePhraseOrAnd('this is the second csv document', with_positions);
    await assertSingle('this is the second csv document');
  }
  // ------------------------------------------------------

  // ------------------------------------------------------
  // Test incremental indexing addition

  // 1, to be deleted later
  await clearInput();
  await typePhraseOrAnd('This URL is invaldi', with_positions);
  await waitNoResults();

  fs.copyFileSync(
    path.join(__dirname, 'incremental_indexing/deletions/404.html'),
    path.join(__dirname, 'input/404.html'),
  );
  runIndexer(`cargo run -p morsels_indexer -- ./e2e/input ./e2e/output --incremental -c ${configFile}`);

  await reloadPage();
  await typePhraseOrAnd('This URL is invaldi', with_positions);
  await assertSingle('this url is invalid');

  // 2, to be updated later
  await clearInput();
  await typePhraseOrAnd('Contributions of any form', with_positions);
  await waitNoResults();

  const contributingHtmlOutputPath = path.join(__dirname, 'input/contributing.html');
  fs.copyFileSync(
    path.join(__dirname, 'incremental_indexing/updates/contributing.html'),
    contributingHtmlOutputPath,
  );
  runIndexer(`cargo run -p morsels_indexer -- ./e2e/input ./e2e/output --incremental -c ${configFile}`);
  
  await reloadPage();
  await typePhraseOrAnd('Contributions of any form', with_positions);
  await assertSingle('contributions of any form');

  // ------------------------------------------------------
  
  // ------------------------------------------------------
  // Test incremental indexing deletion

  fs.rmSync(path.join(__dirname, 'input/404.html'));
  runIndexer(`cargo run -p morsels_indexer -- ./e2e/input ./e2e/output --incremental -c ${configFile}`);
  
  await reloadPage();
  await typePhraseOrAnd('This URL is invaldi', with_positions);
  await waitNoResults();

  // also assert incremental indexing is actually run
  let incrementalIndexInfo = JSON.parse(
    fs.readFileSync(path.join(__dirname, 'output/_incremental_info.json'), 'utf-8'),
  );
  expect(incrementalIndexInfo.num_deleted_docs).toBe(1);

  // ------------------------------------------------------

  // ------------------------------------------------------
  // Test incremental indexing update

  await clearInput();
  await typePhraseOrAnd('Contributions of all forms', with_positions);
  await waitNoResults();

  let contributingHtml = fs.readFileSync(contributingHtmlOutputPath, 'utf-8');
  contributingHtml = contributingHtml.replace(
    'Contributions of any form', 'Contributions of all forms atquejxusd',
  );
  fs.writeFileSync(contributingHtmlOutputPath, contributingHtml);
  runIndexer(`cargo run -p morsels_indexer -- ./e2e/input ./e2e/output --incremental -c ${configFile}`);

  await reloadPage();
  await typePhraseOrAnd('Contributions of any form', with_positions);
  await waitNoResults();

  await clearInput();
  await typePhraseOrAnd('Contributions of all forms', with_positions);
  await assertSingle('contributions of all forms');

  await clearInput();
  await typeText('atquejxusd ');
  await assertSingle('contributions of all forms atquejxusd');

  // also assert incremental indexing is actually run
  incrementalIndexInfo = JSON.parse(
    fs.readFileSync(path.join(__dirname, 'output/_incremental_info.json'), 'utf-8'),
  );
  expect(incrementalIndexInfo.num_deleted_docs).toBe(2);

  // then delete it again
  fs.rmSync(contributingHtmlOutputPath);
  runIndexer(`cargo run -p morsels_indexer -- ./e2e/input ./e2e/output --incremental -c ${configFile}`);
  
  await reloadPage();
  await typePhraseOrAnd('Contributions of any form', with_positions);
  await waitNoResults();

  await clearInput();
  await typePhraseOrAnd('Contributions of all forms', with_positions);
  await waitNoResults();

  await clearInput();
  await typeText('atquejxusd');
  await waitNoResults();

  // also assert incremental indexing is actually run
  incrementalIndexInfo = JSON.parse(
    fs.readFileSync(path.join(__dirname, 'output/_incremental_info.json'), 'utf-8'),
  );
  expect(incrementalIndexInfo.num_deleted_docs).toBe(3);

  // ------------------------------------------------------
};

const cleanup = () => {
  const notFoundFile = path.join(__dirname, 'input/404.html');
  if (fs.existsSync(notFoundFile)) {
    fs.rmSync(notFoundFile);
  }

  const contributingFile = path.join(__dirname, 'input/contributing.html');
  if (fs.existsSync(contributingFile)) {
    fs.rmSync(contributingFile);
  }
};

test('Test with different field and block size configs', async () => {
  cleanup();
  console.log('Starting morsels_config_0 tests');
  await testSuite('e2e/input/morsels_config_0.json');

  // Assert what's cached
  // Slightly different pl_cache_thresholds for the 4 tests
  let outputConfig = JSON.parse(
    fs.readFileSync(path.join(__dirname, 'output/morsels_config.json'), 'utf8'),
  );
  expect(outputConfig.indexingConfig.plNamesToCache).toHaveLength(5);
  expect(outputConfig.indexingConfig.plNamesToCache).toEqual([0, 1, 2, 3, 4]);

  cleanup();
  console.log('Starting morsels_config_1 tests');
  await testSuite('e2e/input/morsels_config_1.json');

  outputConfig = JSON.parse(
    fs.readFileSync(path.join(__dirname, 'output/morsels_config.json'), 'utf8'),
  );
  expect(outputConfig.indexingConfig.plNamesToCache).toHaveLength(2);
  expect(outputConfig.indexingConfig.plNamesToCache).toEqual([0, 1]);

  cleanup();
  console.log('Starting morsels_config_2 tests');
  await testSuite('e2e/input/morsels_config_2.json');

  outputConfig = JSON.parse(
    fs.readFileSync(path.join(__dirname, 'output/morsels_config.json'), 'utf8'),
  );
  expect(outputConfig.indexingConfig.plNamesToCache).toHaveLength(2);
  expect(outputConfig.indexingConfig.plNamesToCache).toEqual([0, 1]);

  cleanup();
  console.log('Starting morsels_config_3 tests');
  await testSuite('e2e/input/morsels_config_3.json');

  outputConfig = JSON.parse(
    fs.readFileSync(path.join(__dirname, 'output/morsels_config.json'), 'utf8'),
  );
  expect(outputConfig.indexingConfig.plNamesToCache).toHaveLength(0);

  // No positions, uses source files to generate result previews
  console.log('Starting morsels_config_4 tests');
  await testSuite('e2e/input/morsels_config_4.json', false);

  outputConfig = JSON.parse(
    fs.readFileSync(path.join(__dirname, 'output/morsels_config.json'), 'utf8'),
  );
  expect(outputConfig.indexingConfig.plNamesToCache).toHaveLength(0);

  process.exit(0);
});

afterAll(cleanup);
