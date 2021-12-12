
const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

jest.setTimeout(3000000);

const INPUT_SELECTOR = '#morsels-search';

async function clearInput() {
  await page.click(INPUT_SELECTOR, { clickCount: 3 });
  await page.keyboard.press('Backspace');
  const numChildren = await page.evaluate(() => {
    return document.getElementById('morsels-search').childNodes.length;
  });
  expect(numChildren).toBe(0);
}

async function typePhrase(phrase) {
  await page.type(INPUT_SELECTOR, `"${phrase}"`, { delay: 20 });
  const inputVal = await page.evaluate(() => document.getElementById('morsels-search').value);
  expect(inputVal).toBe(`"${phrase}"`);
}

async function typeText(text) {
  await page.type(INPUT_SELECTOR, text, { delay: 20 });
  const inputVal = await page.evaluate(() => document.getElementById('morsels-search').value);
  expect(inputVal).toBe(text);
}

async function waitNoResults() {
  try {
    await page.waitForSelector('.morsels-no-results', { timeout: 3000 });
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
    await page.waitForSelector('.morsels-list-item', { timeout: 8000 });

    const result = await page.evaluate(() => {
      const queryResult = document.getElementsByClassName('morsels-list-item');
      return { text: queryResult.length && queryResult[0].textContent, resultCount: queryResult.length };
    });

    expect(result.resultCount).toBe(1);
    expect(result.text.toLowerCase().includes(text)).toBe(true);
  } catch (ex) {
    const output = await page.evaluate(() => document.getElementById('target-mode-el').innerHTML);
    console.error('assertSingle failed, output in target:', output);
    throw ex;
  }
}

async function reloadPage() {
  await jestPuppeteer.resetPage();
  await jestPuppeteer.resetBrowser();
  await page.goto(
    'http://localhost:8080?mode=target&url=http%3A%2F%2Flocalhost%3A3000%2F&resultsPerPage=100',
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

const testSuite = async (configFile) => {
  runIndexer(`cargo run -p morsels_indexer -- ./e2e/input ./e2e/output -c ${configFile}`);

  console.log('Ran full indexer run');

  await reloadPage();

  // ------------------------------------------------------
  // Various basic tests on docid=0
  await typeText('npm AND run AND dev AND installmdbook');
  await assertSingle('use the npm run dev script');

  await clearInput();
  await typeText('"npm run dev" AND (installmdbook 8080)');
  await assertSingle('use the npm run dev script');

  await clearInput();
  await typeText('npm AND run AND dev AND nonexistentterm');
  await waitNoResults();

  await clearInput();
  await typePhrase('Once you have you test files');
  await assertSingle('once you have you test files');
  // ------------------------------------------------------

  // ------------------------------------------------------
  // Simple phrase query test on another docid
  await clearInput();
  await typePhrase('forenote on mobile device detection');
  await assertSingle('forenote on mobile device detection');
  // ------------------------------------------------------

  // ------------------------------------------------------
  // JsonLoader tests
  await clearInput();
  await typePhrase('Lorem Ipsum is simply dummy text');
  await assertSingle('lorem ipsum is simply dummy text');

  await clearInput();
  await typePhrase('test many json 2');
  await assertSingle('test many json 2');
  // ------------------------------------------------------

  // ------------------------------------------------------
  // CsvLoader tests
  await clearInput();
  await typePhrase('this is the second csv document');
  await assertSingle('this is the second csv document');
  // ------------------------------------------------------

  // ------------------------------------------------------
  // Test dynamic indexing addition

  // 1, to be deleted later
  await clearInput();
  await typePhrase('This URL is invalid');
  await waitNoResults();

  fs.copyFileSync(
    path.join(__dirname, 'dynamic_indexing/deletions/404.html'),
    path.join(__dirname, 'input/404.html'),
  );
  runIndexer(`cargo run -p morsels_indexer -- ./e2e/input ./e2e/output --dynamic -c ${configFile}`);

  await reloadPage();
  await typePhrase('This URL is invalid');
  await assertSingle('this url is invalid');

  // 2, to be updated later
  await clearInput();
  await typePhrase('Contributions of any form');
  await waitNoResults();

  const contributingHtmlOutputPath = path.join(__dirname, 'input/contributing.html');
  fs.copyFileSync(
    path.join(__dirname, 'dynamic_indexing/updates/contributing.html'),
    contributingHtmlOutputPath,
  );
  runIndexer(`cargo run -p morsels_indexer -- ./e2e/input ./e2e/output --dynamic -c ${configFile}`);
  
  await reloadPage();
  await typePhrase('Contributions of any form');
  await assertSingle('contributions of any form');

  // ------------------------------------------------------
  
  // ------------------------------------------------------
  // Test dynamic indexing deletion

  fs.rmSync(path.join(__dirname, 'input/404.html'));
  runIndexer(`cargo run -p morsels_indexer -- ./e2e/input ./e2e/output --dynamic -c ${configFile}`);
  
  await reloadPage();
  await typePhrase('This URL is invalid');
  await waitNoResults();

  // also assert dynamic indexing is actually run
  let dynamicIndexInfo = JSON.parse(
    fs.readFileSync(path.join(__dirname, 'output/_dynamic_index_info.json'), 'utf-8'),
  );
  expect(dynamicIndexInfo.num_deleted_docs).toBe(1);

  // ------------------------------------------------------

  // ------------------------------------------------------
  // Test dynamic indexing update

  await clearInput();
  await typePhrase('Contributions of all forms');
  await waitNoResults();

  let contributingHtml = fs.readFileSync(contributingHtmlOutputPath, 'utf-8');
  contributingHtml = contributingHtml.replace(
    'Contributions of any form', 'Contributions of all forms atquejxusd',
  );
  fs.writeFileSync(contributingHtmlOutputPath, contributingHtml);
  runIndexer(`cargo run -p morsels_indexer -- ./e2e/input ./e2e/output --dynamic -c ${configFile}`);

  await reloadPage();
  await typePhrase('Contributions of any form');
  await waitNoResults();

  await clearInput();
  await typePhrase('Contributions of all forms');
  await assertSingle('contributions of all forms');

  await clearInput();
  await typeText('atquejxusd ');
  await assertSingle('contributions of all forms atquejxusd');

  // also assert dynamic indexing is actually run
  dynamicIndexInfo = JSON.parse(
    fs.readFileSync(path.join(__dirname, 'output/_dynamic_index_info.json'), 'utf-8'),
  );
  expect(dynamicIndexInfo.num_deleted_docs).toBe(2);

  // then delete it again
  fs.rmSync(contributingHtmlOutputPath);
  runIndexer(`cargo run -p morsels_indexer -- ./e2e/input ./e2e/output --dynamic -c ${configFile}`);
  
  await reloadPage();
  await typePhrase('Contributions of any form');
  await waitNoResults();

  await clearInput();
  await typePhrase('Contributions of all forms');
  await waitNoResults();

  await clearInput();
  await typeText('atquejxusd');
  await waitNoResults();

  // also assert dynamic indexing is actually run
  dynamicIndexInfo = JSON.parse(
    fs.readFileSync(path.join(__dirname, 'output/_dynamic_index_info.json'), 'utf-8'),
  );
  expect(dynamicIndexInfo.num_deleted_docs).toBe(3);

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
  cleanup();
  console.log('Starting morsels_config_1 tests');
  await testSuite('e2e/input/morsels_config_1.json');
  cleanup();
  console.log('Starting morsels_config_2 tests');
  await testSuite('e2e/input/morsels_config_2.json');
  cleanup();
  console.log('Starting morsels_config_3 tests');
  await testSuite('e2e/input/morsels_config_3.json');
  process.exit(0);
});

afterAll(cleanup);
