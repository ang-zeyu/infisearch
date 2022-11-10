const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

const INPUT_SELECTOR = '#morsels-search';

async function clearInput() {
  await page.click(INPUT_SELECTOR, { clickCount: 3 });
  await page.keyboard.press('Backspace');
  await page.waitForSelector('#target-mode-el .morsels-blank');
  await page.waitForSelector('#target-mode-el > [role="listbox"]');
  const numChildren = await page.evaluate(() => {
    const listbox = document.querySelector('#target-mode-el > [role="listbox"]');
    return listbox && listbox.childNodes.length;
  });
  expect(numChildren).toBe(0);
}

async function typePhraseOrAnd(phrase, with_positions) {
  await clearInput();

  if (with_positions) {
    console.log(`Typing phrase '${phrase}'`);
    await page.type(INPUT_SELECTOR, `"${phrase}"`);
    const inputVal = await page.evaluate(() => document.getElementById('morsels-search').value);
    expect(inputVal).toBe(`"${phrase}"`);
  } else {
    const query = phrase.split(/\s+/g).map((term) => `+${term}`).join(' ') + ' ';
    console.log(`Falling back to AND '${query}'`);
    await page.type(INPUT_SELECTOR, query);
    const inputVal = await page.evaluate(() => document.getElementById('morsels-search').value);
    expect(inputVal).toBe(query);
  }
}

async function typeText(text) {
  await clearInput();

  console.log(`Typing text '${text}'`);
  await page.type(INPUT_SELECTOR, text);
  const inputVal = await page.evaluate(() => document.getElementById('morsels-search').value);
  expect(inputVal).toBe(text);
}

async function waitNoResults() {
  try {
    await page.waitForSelector('#target-mode-el .morsels-header .morsels-results-found', { timeout: 10000 });
    const headerText = await page.evaluate(() => {
      const header = document.querySelector('#target-mode-el .morsels-header');
      return header && header.textContent;
    });
    expect(typeof headerText).toBe('string');
    expect(headerText.trim().startsWith('0 results found')).toBe(true);
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
    await page.waitForSelector('#target-mode-el .morsels-list-item', { timeout: 60000 });

    const result = await page.evaluate(() => {
      const queryResult = document.querySelectorAll('#target-mode-el .morsels-list-item');
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
    console.error(
      'assertSingle failed,\n'
        + '----------------\nHTML in target:\n'
        + output.html
        + '----------------\ntext in target:\n'
        + output.text
        + '----------------\nexpected text:\n'
        + text,
    );
    throw ex;
  }
}

async function assertMultiple(texts, count) {
  try {
    await page.waitForSelector('#target-mode-el .morsels-list-item', { timeout: 60000 });

    const result = await page.evaluate(() => {
      const queryResult = document.querySelectorAll('#target-mode-el .morsels-list-item');
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

function expectNumDeletedDocs(n) {
  const incrementalIndexInfo = JSON.parse(
    fs.readFileSync(path.join(__dirname, 'output/_incremental_info.json'), 'utf-8'),
  );
  expect(incrementalIndexInfo.num_deleted_docs).toBe(n);
}

async function setActiveClass(selector) {
  await page.waitForSelector(selector);

  const activeSelector = `${selector}.active`;
  const isDropdownExpanded = await page.evaluate((s) => !!document.querySelector(s), activeSelector);

  if (!isDropdownExpanded) {
    await page.evaluate((s) => document.querySelector(s).click(), selector);
    await page.waitForSelector(activeSelector);
  }
}

async function clickCheckbox(selector, active) {
  await page.waitForSelector(selector);

  const activeSelector = `${selector}${active ? ':checked' : ':not(:checked)'}`;
  const inActiveState = await page.evaluate((s) => !!document.querySelector(s), activeSelector);

  if (!inActiveState) {
    await page.evaluate((s) => document.querySelector(s).click(), selector);
    await page.waitForSelector(activeSelector);
  }
}

async function selectFilters(enumsToValues, unspecifiedIsChecked = true) {
  // Expand the filters
  await setActiveClass('#target-mode-el button.morsels-filters');

  const allHeaders = await page.evaluate(() => {
    const options = document.querySelectorAll('#target-mode-el .morsels-filter-header');
    return Array.from(options).map((el) => el.textContent);
  });

  // Expand all headers
  for (let headerIdx = 1; headerIdx <= allHeaders.length; headerIdx += 1) {
    await setActiveClass(
      `#target-mode-el .morsels-filter:nth-child(${headerIdx}) .morsels-filter-header`,
    );
  }

  // Click the options
  for (let headerIdx = 1; headerIdx <= allHeaders.length; headerIdx += 1) {
    const headerText = allHeaders[headerIdx - 1];
    const specifiedValues = enumsToValues[headerText];

    const optionsContainer =
      `#target-mode-el .morsels-filter:nth-child(${headerIdx}) [role="listbox"]`;

    const uiValues = await page.evaluate((optionsContainerSelector) => {
      const options = document.querySelectorAll(optionsContainerSelector + ' .morsels-filter-opt');
      return Array.from(options).map((el) => el.textContent.trim());
    }, optionsContainer);

    for (let optionIdx = 1; optionIdx <= uiValues.length; optionIdx += 1) {
      // Select all options in unspecified headers
      // and specified options in specified headers
      const active = (!specifiedValues && unspecifiedIsChecked)
        || specifiedValues.includes(uiValues[optionIdx - 1]);

      await clickCheckbox(
        optionsContainer
        + ` .morsels-filter-opt:nth-child(${optionIdx}) input[type="checkbox"]`,
        active,
      );
    }
  }
}

async function reloadPage(lang = 'ascii') {
  await jestPuppeteer.resetPage();
  await jestPuppeteer.resetBrowser();
  
  page
    .on('console', message =>
      console.log(`${message.type()} ${message.text()}`))
    .on('error', (ex) => console.error('Unexpected (1): ' + ex))
    .on('pageerror', ({ message }) => console.error('Unexpected (2): ' + message));
  
  const url = `http://localhost:8080/basic-theme_${lang}-lang.html?mode=target`
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
  
function runFullIndex(configFile) {
  runIndexer(`cargo run -p morsels_indexer -- ./e2e/input ./e2e/output -c ${configFile}`);
  console.log('Ran full indexer run');
}
  
function runIncrementalIndex(configFile) {
  runIndexer(`cargo run -p morsels_indexer -- ./e2e/input ./e2e/output -c ${configFile} --incremental`);
  console.log('Ran incremental indexer run');
}

module.exports = {
  typePhraseOrAnd,
  typeText,
  waitNoResults,
  assertSingle,
  assertMultiple,
  expectNumDeletedDocs,
  reloadPage,
  runFullIndex,
  runIncrementalIndex,
  selectFilters,
};
