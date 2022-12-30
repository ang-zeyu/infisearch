const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

const INPUT_SELECTOR = '#infi-search';

async function clearInput() {
  await page.click(INPUT_SELECTOR, { clickCount: 3 });
  await page.keyboard.press('Backspace');
  await page.waitForSelector('#target-mode-el .infi-blank');
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
    const inputVal = await page.evaluate(() => document.getElementById('infi-search').value);
    expect(inputVal).toBe(`"${phrase}"`);
  } else {
    const query = phrase.split(/\s+/g).map((term) => `+${term}`).join(' ') + ' ';
    console.log(`Falling back to AND '${query}'`);
    await page.type(INPUT_SELECTOR, query);
    const inputVal = await page.evaluate(() => document.getElementById('infi-search').value);
    expect(inputVal).toBe(query);
  }
}

async function typeText(text) {
  await clearInput();

  console.log(`Typing text '${text}'`);
  await page.type(INPUT_SELECTOR, text);
  const inputVal = await page.evaluate(() => document.getElementById('infi-search').value);
  expect(inputVal).toBe(text);
}

async function waitNoResults() {
  try {
    await page.waitForSelector('#target-mode-el .infi-header .infi-results-found', { timeout: 10000 });
    const headerText = await page.evaluate(() => {
      const header = document.querySelector('#target-mode-el .infi-header');
      return header && header.textContent;
    });
    expect(typeof headerText).toBe('string');
    expect(headerText.trim().startsWith('0 results found')).toBe(true);
  } catch (ex) {
    const output = await page.evaluate(() => document.getElementById('target-mode-el').innerHTML);
    console.error('waitNoResults failed, output in target:', output);
    console.error('input element text:');
    const inputElText = await page.evaluate(() => document.getElementById('infi-search').value);
    console.error(inputElText);
    throw ex;
  }
}

async function assertSingle(text) {
  try {
    await page.waitForSelector('#target-mode-el .infi-list-item', { timeout: 60000 });

    const result = await page.evaluate(() => {
      const queryResult = document.querySelectorAll('#target-mode-el .infi-list-item');
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
    await page.waitForSelector('#target-mode-el .infi-list-item', { timeout: 60000 });

    const result = await page.evaluate(() => {
      const queryResult = document.querySelectorAll('#target-mode-el .infi-list-item');
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

async function assertMultipleOrdered(texts) {
  try {
    await page.waitForSelector('#target-mode-el .infi-list-item', { timeout: 60000 });

    const result = await page.evaluate(() => {
      const queryResult = document.querySelectorAll('#target-mode-el .infi-list-item');
      return {
        texts: Array.from(queryResult).map((el) => el.textContent),
        resultCount: queryResult.length,
      };
    });

    expect(result.resultCount).toBe(texts.length);
    texts.forEach((text, idx) => {
      expect(
        result.texts[idx].toLowerCase().includes(text.toLowerCase()),
      ).toBe(true);
    });
  } catch (ex) {
    const output = await page.evaluate(() => {
      return {
        html: document.getElementById('target-mode-el').innerHTML,
        text: document.getElementById('target-mode-el').textContent,
      };
    });
    console.error('assertMultipleOrdered failed, html in target:', output.html);
    console.error('assertMultipleOrdered failed, text in target:', output.text);
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
  // Expand the filters if needed
  await setActiveClass('#target-mode-el button.infi-filters');

  const allHeaders = await page.evaluate(() => {
    const options = document.querySelectorAll('#target-mode-el .infi-multi-header');
    return Array.from(options).map((el) => el.textContent);
  });

  // Click the options
  siblingSelector = '';
  for (const headerText of allHeaders) {
    siblingSelector += '+div';

    // Expand the header if needed
    await setActiveClass(
      `#target-mode-el .infi-sep${siblingSelector} .infi-multi-header`,
    );

    const specifiedValues = enumsToValues[headerText];

    const optionsContainer =
      `#target-mode-el .infi-sep${siblingSelector} [role="listbox"]`;

    const uiValues = await page.evaluate((optionsContainerSelector) => {
      const options = document.querySelectorAll(optionsContainerSelector + ' .infi-multi');
      return Array.from(options).map((el) => el.textContent.trim());
    }, optionsContainer);

    for (let optionIdx = 1; optionIdx <= uiValues.length; optionIdx += 1) {
      // Select all options in unspecified headers
      // and specified options in specified headers
      const active = (!specifiedValues && unspecifiedIsChecked)
        || specifiedValues.includes(uiValues[optionIdx - 1]);

      await clickCheckbox(
        optionsContainer
        + ` .infi-multi:nth-child(${optionIdx}) input[type="checkbox"]`,
        active,
      );
    }
  }
}

async function setNumericFilter(filterName, min, max) {
  // Expand the filters if needed
  await setActiveClass('#target-mode-el button.infi-filters');

  const headerIdx = await page.evaluate((name) => {
    const options = document.querySelectorAll('#target-mode-el .infi-min-max .infi-filter-header');
    return Array.from(options).findIndex((el) => el.textContent === name);
  }, filterName);

  if (headerIdx === -1) {
    throw new Error('Invalid filter name');
  }

  const minMaxSelector = '#target-mode-el .infi-sep' + '+.infi-min-max'.repeat(headerIdx + 1);
  const minSelector = minMaxSelector + ' .infi-filter-header+label input.infi-minmax';
  const maxSelector = minMaxSelector + ' .infi-filter-header+label+label input.infi-minmax';

  await page.evaluate((minSel, maxSel, minValue, maxValue) => {
    const minInput = document.querySelector(minSel);
    const maxInput = document.querySelector(maxSel);

    minInput.value = minValue;
    minInput.dispatchEvent(new KeyboardEvent('change'));
    maxInput.value = maxValue;
    maxInput.dispatchEvent(new KeyboardEvent('change'));
  }, minSelector, maxSelector, min, max);
}

async function setSortOption(option) {
  // Expand the filters if needed
  await setActiveClass('#target-mode-el button.infi-filters');
  await clearInput();
  await page.select('#target-mode-el .infi-sort', option);
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
  await expect(page.title()).resolves.toMatch('InfiSearch');
}

function runIndexer(command) {
  execSync(command, {
    env: { RUST_BACKTRACE: 1, ...process.env },
    stdio: 'inherit',
  });
}
  
function runFullIndex(configFile) {
  runIndexer(`cargo run -p infisearch -- ./e2e/input ./e2e/output -c ${configFile}`);
  console.log('Ran full indexer run');
}
  
function runIncrementalIndex(configFile) {
  runIndexer(`cargo run -p infisearch -- ./e2e/input ./e2e/output -c ${configFile} --incremental`);
  console.log('Ran incremental indexer run');
}

module.exports = {
  typePhraseOrAnd,
  typeText,
  waitNoResults,
  assertSingle,
  assertMultiple,
  assertMultipleOrdered,
  expectNumDeletedDocs,
  reloadPage,
  runFullIndex,
  runIncrementalIndex,
  selectFilters,
  setNumericFilter,
  setSortOption,
};
