const fs = require('fs');
const path = require('path');

const { addFilesTest, cleanupAddFilesTests } = require('./addfiles');
const {
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
  setNumericFilter,
  setSortOption,
  assertMultipleOrdered,
} = require('./utils');

jest.setTimeout(3000000);

beforeAll(async () => {
  // Wait for webpack-dev-server to complete before running indexer
  await reloadPage();
});

const GETTING_STARTED_TITLE = 'Getting Started - Morsels Documentation';
const GETTING_STARTED_MDBOOK_TITLE = 'Mdbook - Morsels Documentation';
const GETTING_STARTED_OTHERS_TITLE = 'Others - Morsels Documentation';
const INDEXER_CONFIG_PAGE_TITLE = 'Indexing Configuration - Morsels Documentation';
const DEVELOPING_PAGE_TITLE = 'Developing - Morsels Documentation';

function replaceIntroductionMood(mood) {
  const fp = path.join(__dirname, './input/introduction.html');
  const inputHTML = fs.readFileSync(fp, { encoding: 'utf-8' });
  const moodMatcher = new RegExp('data-infisearch-mood="[a-z]*"');
  const replacedHTML = inputHTML.replace(moodMatcher, `data-infisearch-mood="${mood}"`);
  fs.writeFileSync(fp, replacedHTML);
}

function replaceGettingStartedOthersDate(dateString) {
  const fp = path.join(__dirname, './input/getting_started_others.html');
  const inputHTML = fs.readFileSync(fp, { encoding: 'utf-8' });
  const dateMatcher = new RegExp('data-infisearch-dateposted=".*?"');
  const replacedHTML = inputHTML.replace(dateMatcher, `data-infisearch-dateposted="${dateString}"`);
  fs.writeFileSync(fp, replacedHTML);
}

const testSuite = async (configFile, with_positions, with_filters) => {
  runFullIndex(configFile);

  const lang = JSON.parse(
    fs.readFileSync(path.join(__dirname, '..', configFile), 'utf8'),
  ).lang_config.lang;

  await reloadPage(lang);

  // ------------------------------------------------------
  // Various basic tests on docid=0
  await typeText('+npm +run +dev +installmdbook');
  await assertSingle('use the npm run dev script');

  if (with_positions) {
    await typeText('+"npm run dev" +(installmdbook 8080)');
    await assertSingle('use the npm run dev script');
  }

  await typeText('(+npm +run +dev) +nonexistentterm');
  await waitNoResults();

  await typeText('npm -run -packages');
  await waitNoResults();

  await typeText('+npm +run +setup');
  await assertSingle('npm run setup');

  for (const query of ['(+npm +run +setup) -npm', '(+npm +run +setup) -(+npm run +setup)']) {
    await typeText(query);
    await waitNoResults();
  }

  if (with_positions) {
    await typeText('body:"Once you have you test files"');
    await assertSingle('once you have you test files');
  
    await typeText('title:"Once you have you test files"');
    await waitNoResults();
  
    await typeText('title:"Developing - Morsels Documentation"');
    await assertSingle('developing - morsels documentation');
  
    await typeText('heading:"Developing - Morsels Documentation"');
    await waitNoResults();
  }
  // ------------------------------------------------------

  // ------------------------------------------------------
  await typePhraseOrAnd('Test Heading Only SERP', with_positions);
  await assertSingle('Test Heading Only SERP');
  // ------------------------------------------------------

  // ------------------------------------------------------
  await typeText('testMetaDescription ', with_positions);
  await assertSingle('testMetaDescription');

  await typeText('testMetaKeywords ', with_positions);
  await assertSingle('testMetaDescription');
  // ------------------------------------------------------

  // ------------------------------------------------------
  // Simple phrase query test on another docid
  await typePhraseOrAnd('forenote on mobile device detection', with_positions);
  await assertSingle('forenote on mobile device detection');
  // ------------------------------------------------------

  // ------------------------------------------------------
  // data-infisearch-ignore test
  await typePhraseOrAnd('I should be ignored through this randomuniquewordnkashdcfd', with_positions);
  await waitNoResults();
  // ------------------------------------------------------

  // ------------------------------------------------------
  if (with_filters) {
    // ------------------------------------------------------
    // Multi-select tests
    /*
     enumtestvalidationstringone - sunny delightful
     enumtestvalidationstringtwo - gloomy sad
     enumtestvalidationstringthree - sunny None
     enumtestvalidationstringfour - None sad
     enumtestvalidationstringfive - None delightful
    */

    for (const introductionMood of ['delightful', 'ecstatic', 'None']) {
      await selectFilters({ Weather: [] });
      await typeText('~returnalldocs');
      await waitNoResults();

      await selectFilters({ Mood: [] });
      await typeText('~returnalldocs');
      await waitNoResults();

      await selectFilters({ Weather: ['gloomy'] });
      await typeText('~returnalldocs');
      await assertSingle(GETTING_STARTED_OTHERS_TITLE);
  
      await selectFilters({ Weather: ['sunny'] });
      await typeText('enumteststring ');
      await assertMultiple(['enumtestvalidationstringone', 'enumtestvalidationstringthree'], 2); 
  
      await selectFilters({ Weather: ['sunny'], Mood: [] });
      await typeText('enumteststring ');
      await waitNoResults();
  
      await selectFilters({ Weather: [], Mood: ['None'] });
      await typeText('enumteststring ');
      await waitNoResults();
  
      await selectFilters({ Mood: ['None'] });
      await typeText('enumteststring ');
      if (introductionMood === 'None') {
        await assertMultiple([
          'enumtestvalidationstringthree',
          'enumtestvalidationstringfive',
        ], 2);
      } else {
        await assertSingle('enumtestvalidationstringthree');
      }
  
      await selectFilters({ Mood: ['sad', 'delightful'] });
      await typeText('enumteststring ');
      if (introductionMood === 'delightful') {
        await assertMultiple([
          'enumtestvalidationstringone',
          'enumtestvalidationstringtwo',
          'enumtestvalidationstringfour',
          'enumtestvalidationstringfive',
        ], 4);
      } else {
        await assertMultiple([
          'enumtestvalidationstringone',
          'enumtestvalidationstringtwo',
          'enumtestvalidationstringfour',
        ], 3);
      }
  
      await selectFilters({ Weather: ['sunny'], Mood: ['delightful'] });
      await typeText('enumteststring ');
      await assertSingle('enumtestvalidationstringone');
  
      await selectFilters({ Weather: ['None'], Mood: [ introductionMood ] });
      await typeText('enumteststring ');
      await assertSingle('enumtestvalidationstringfive');
  
      await selectFilters({ Weather: ['gloomy'], Mood: ['sad'] });
      await typeText('enumteststring ');
      await assertSingle('enumtestvalidationstringtwo');
  
      await selectFilters({ Weather: ['sunny', 'gloomy'], Mood: ['sad'] });
      await typeText('enumteststring ');
      await assertSingle('enumtestvalidationstringtwo');
  
      await selectFilters({ Weather: ['sunny', 'gloomy'], Mood: ['None'] });
      await typeText('enumteststring ');
      await assertSingle('enumtestvalidationstringthree');
  
      await selectFilters({ Weather: ['None'], Mood: ['None'] });
      await typeText('enumteststring ');
      if (introductionMood === 'None') {
        await assertSingle('enumtestvalidationstringfive');
      } else {
        await waitNoResults();
      }
  
      await selectFilters({ Weather: ['sunny'], Mood: ['sad'] });
      await typeText('enumteststring ');
      await waitNoResults();
  
      await selectFilters({ Weather: ['sunny', 'None'], Mood: ['sad', 'delightful'] });
      await typeText('enumteststring ');
      if (introductionMood === 'delightful') {
        await assertMultiple([
          'enumtestvalidationstringone',
          'enumtestvalidationstringfour',
          'enumtestvalidationstringfive',
        ], 3);
      } else {
        await assertMultiple([
          'enumtestvalidationstringone',
          'enumtestvalidationstringfour',
        ], 2);
      }


      if (introductionMood === 'delightful') {
        replaceIntroductionMood('ecstatic');
        expectNumDeletedDocs(0);
        runIncrementalIndex(configFile);
        expectNumDeletedDocs(1);
        await reloadPage(lang);
      } else if (introductionMood === 'ecstatic') {
        // Empty strings are discarded
        // So this doc should be assigned the default enum value
        replaceIntroductionMood('');
        runIncrementalIndex(configFile);
        expectNumDeletedDocs(2);
        await reloadPage(lang);
      }
    }
    // ------------------------------------------------------
    // Numeric filter tests

    await reloadPage(lang);

    await setNumericFilter('Price', 200, 100);
    await typeText('~returnalldocs');
    await waitNoResults();

    await setNumericFilter('Price', -100, -100);
    await typeText('~returnalldocs');
    await assertSingle(GETTING_STARTED_TITLE);

    await setNumericFilter('Price', -100, 0);
    await typeText('~returnalldocs');
    await assertSingle(GETTING_STARTED_TITLE);

    await setNumericFilter('Price', 0, 0);
    await typeText('~returnalldocs');
    await waitNoResults();

    await setNumericFilter('Price', 100, 100);
    await typeText('~returnalldocs');
    await assertSingle(GETTING_STARTED_MDBOOK_TITLE);

    await setNumericFilter('Price', 100, 199);
    await typeText('~returnalldocs');
    await assertSingle(GETTING_STARTED_MDBOOK_TITLE);

    await setNumericFilter('Price', 100, 200);
    await typeText('~returnalldocs');
    await assertMultiple([GETTING_STARTED_MDBOOK_TITLE, GETTING_STARTED_OTHERS_TITLE], 2);

    await setNumericFilter('Price', 100, '');
    await typeText('~returnalldocs');
    await assertMultiple([GETTING_STARTED_MDBOOK_TITLE, GETTING_STARTED_OTHERS_TITLE], 2);

    await setNumericFilter('Price', 200, 200);
    await typeText('~returnalldocs');
    await assertSingle(GETTING_STARTED_OTHERS_TITLE);

    await setNumericFilter('Price', 200, 201);
    await typeText('~returnalldocs');
    await assertSingle(GETTING_STARTED_OTHERS_TITLE);

    await setNumericFilter('Price', '', '');

    // Negative UNIX timestamp test
    await setNumericFilter('Date Posted', '1950-12-11T00:00', '1950-12-12T11:59');
    await typeText('~returnalldocs');
    await assertSingle(DEVELOPING_PAGE_TITLE);

    await setNumericFilter('Date Posted', '2022-12-11T00:00', '2022-12-12T11:59');
    await typeText('~returnalldocs');
    await assertSingle(GETTING_STARTED_OTHERS_TITLE);

    await setNumericFilter('Date Posted', '2022-12-11T00:00', '2022-12-15T23:59');
    await typeText('~returnalldocs');
    await assertMultiple([INDEXER_CONFIG_PAGE_TITLE, GETTING_STARTED_TITLE, GETTING_STARTED_OTHERS_TITLE], 3);

    replaceGettingStartedOthersDate('2022 Dec 13 12:08 +0000');
    runIncrementalIndex(configFile);
    expectNumDeletedDocs(3);
    await reloadPage(lang);

    // Negative UNIX timestamp test
    await setNumericFilter('Date Posted', '1950-12-11T00:00', '1950-12-12T11:59');
    await typeText('~returnalldocs');
    await assertSingle(DEVELOPING_PAGE_TITLE);

    await setNumericFilter('Date Posted', '2022-12-11T00:00', '2022-12-12T11:59');
    await typeText('~returnalldocs');
    await waitNoResults();

    await setNumericFilter('Date Posted', '2022-12-11T00:00', '2022-12-15T23:59');
    await typeText('~returnalldocs');
    await assertMultiple([INDEXER_CONFIG_PAGE_TITLE, GETTING_STARTED_TITLE, GETTING_STARTED_OTHERS_TITLE], 3);

    // ------------------------------------------------------
    // Sort option tests

    // <->0 means high (latest) to low (oldest)
    await setSortOption('dateposted<->0');
    await typeText('~returnalldocs');
    await assertMultipleOrdered([
      INDEXER_CONFIG_PAGE_TITLE, GETTING_STARTED_TITLE, GETTING_STARTED_OTHERS_TITLE,
    ]);

    // <->1 means low (oldest) to high (latest)
    await setSortOption('dateposted<->1');
    await typeText('~returnalldocs');
    await assertMultipleOrdered([
      GETTING_STARTED_OTHERS_TITLE, GETTING_STARTED_TITLE, INDEXER_CONFIG_PAGE_TITLE,
    ]);

    await reloadPage(lang);
    await setNumericFilter('Price', 100, 200);
    await typeText('~returnalldocs');
    await assertMultiple([GETTING_STARTED_MDBOOK_TITLE, GETTING_STARTED_OTHERS_TITLE], 2);

    await setSortOption('price<->0');
    await typeText('~returnalldocs');
    await assertMultipleOrdered([GETTING_STARTED_OTHERS_TITLE, GETTING_STARTED_MDBOOK_TITLE]);

    await setSortOption('price<->1');
    await typeText('~returnalldocs');
    await assertMultipleOrdered([GETTING_STARTED_MDBOOK_TITLE, GETTING_STARTED_OTHERS_TITLE]);

    runFullIndex(configFile);
    await reloadPage(lang);
  }
  // ------------------------------------------------------

  // ------------------------------------------------------
  // Spelling correction tests
  await typeText('fornote');
  await assertMultiple([
    'forenote on stop words',
    'forenote on mobile device detection',
  ], 2);

  await typePhraseOrAnd('middle fornote on stop words');
  await assertSingle('middle forenote on stop words');
  // ------------------------------------------------------

  // ------------------------------------------------------
  // (Automatic) term expansion / prefix search tests
  await typeText('foreno');
  await assertMultiple([
    'forenote on stop words',
    'forenote on mobile device detection',
  ], 2);

  /*
  Using "detec" here specifically also does an integration test with
  spelling correction; Terms that are suffix-searched should remove
  any spelling corrections.
  */

  await typeText('detec ');
  await assertMultiple(['date'], 2);

  // No ending space triggers an automatic prefix search
  const expectedPrefixResults = [
    'detecting such terms',
    'the change detection',
    'detectedd as per the earlier section',
    'mobile device detection',
  ];
  await typeText('detec');
  await assertMultiple(expectedPrefixResults, 2);

  await typeText('detec* ');
  await assertMultiple(expectedPrefixResults, 2);

  await typeText('detec*');
  await assertMultiple(expectedPrefixResults, 2);

  await typeText('+detec* +deleted +changed +added +dynamic');
  await assertSingle('detecting deleted, changed, or added');

  /*
  Integration test with field filters.
  */
  await typeText('title:star');
  await assertSingle('started');

  // ------------------------------------------------------

  // ------------------------------------------------------
  // JsonLoader tests
  await typePhraseOrAnd('Lorem Ipsum is simply dummy text', with_positions);
  await assertSingle('lorem ipsum is simply dummy text');

  await typePhraseOrAnd('test many json 2', with_positions);
  await assertSingle('test many json 2');
  // ------------------------------------------------------

  // ------------------------------------------------------
  // CsvLoader tests
  await typePhraseOrAnd('this is the second csv document verylongrandomtermabcdefg', with_positions);
  await assertSingle('this is the second csv document verylongrandomtermabcdefg');
  // ------------------------------------------------------

  // ------------------------------------------------------
  // PdfLoader tests
  await typePhraseOrAnd('this is a pdf document', with_positions);
  await assertSingle('this is a pdf document');
  // ------------------------------------------------------

  // ------------------------------------------------------
  // _add_files tests
  await addFilesTest(with_positions, configFile);
  // ------------------------------------------------------

  // ------------------------------------------------------
  // Test incremental indexing addition

  // Start with a fresh slate
  runFullIndex(configFile);

  // 1, to be deleted later
  await reloadPage(lang);
  await typePhraseOrAnd('This URL is invaldi', with_positions);
  await waitNoResults();

  fs.copyFileSync(
    path.join(__dirname, 'incremental_indexing/deletions/404.html'),
    path.join(__dirname, 'input/404.html'),
  );
  runIncrementalIndex(configFile);

  await reloadPage(lang);
  await typePhraseOrAnd('This URL is invaldi', with_positions);
  await assertSingle('this url is invalid');

  // 2, to be updated later
  await typePhraseOrAnd('Contributions of any form', with_positions);
  await waitNoResults();

  const contributingHtmlOutputPath = path.join(__dirname, 'input/contributing.html');
  fs.copyFileSync(
    path.join(__dirname, 'incremental_indexing/updates/contributing.html'),
    contributingHtmlOutputPath,
  );
  runIncrementalIndex(configFile);
  
  await reloadPage(lang);
  await typePhraseOrAnd('Contributions of any form', with_positions);
  await assertSingle('contributions of any form');

  // ------------------------------------------------------
  
  // ------------------------------------------------------
  // Test incremental indexing deletion

  expectNumDeletedDocs(0);

  fs.rmSync(path.join(__dirname, 'input/404.html'));
  runIncrementalIndex(configFile);
  
  await reloadPage(lang);
  await typePhraseOrAnd('This URL is invaldi', with_positions);
  await waitNoResults();

  expectNumDeletedDocs(1);

  // ------------------------------------------------------

  // ------------------------------------------------------
  // Test incremental indexing update

  await typePhraseOrAnd('Contributions of all forms', with_positions);
  await waitNoResults();

  let contributingHtml = fs.readFileSync(contributingHtmlOutputPath, 'utf-8');
  contributingHtml = contributingHtml.replace(
    'Contributions of any form', 'Contributions of all forms atquejxusd',
  );
  fs.writeFileSync(contributingHtmlOutputPath, contributingHtml);
  runIncrementalIndex(configFile);

  await reloadPage(lang);
  await typePhraseOrAnd('Contributions of any form', with_positions);
  await waitNoResults();

  await typePhraseOrAnd('Contributions of all forms', with_positions);
  await assertSingle('contributions of all forms');

  await typeText('atquejxusd ');
  await assertSingle('contributions of all forms atquejxusd');

  // also assert incremental indexing is actually run
  expectNumDeletedDocs(2);

  // then delete it again
  fs.rmSync(contributingHtmlOutputPath);
  runIncrementalIndex(configFile);
  
  await reloadPage(lang);
  await typePhraseOrAnd('Contributions of any form', with_positions);
  await waitNoResults();

  await typePhraseOrAnd('Contributions of all forms', with_positions);
  await waitNoResults();

  await typeText('atquejxusd');
  await waitNoResults();

  // also assert incremental indexing is actually run
  expectNumDeletedDocs(3);

  // ------------------------------------------------------
};

async function testChinese() {
  console.log('Starting chinese tests');
  await reloadPage('chinese');

  await typePhraseOrAnd('zzz今天 zzz 我x们', true);
  await assertSingle('Test that multilingual positions are encoded correctly');

  await typePhraseOrAnd('今天我们', true);
  await assertSingle('Monolingual test');

  // Test traditional - chinese conversion
  await typeText('㑳');
  await assertSingle('㑇');
}

async function testTokenizerOptions(configFile) {
  console.log('Starting stop words tests');

  runFullIndex(configFile);

  const sourceConfigFile = JSON.parse(
    fs.readFileSync(path.join(__dirname, '..', configFile), 'utf8'),
  );

  await reloadPage(sourceConfigFile.lang_config.lang);

  // ------------------------------------------------------
  // Stop words are only completely ignored if this is true
  const stopWordsRemoved = sourceConfigFile.lang_config.options.ignore_stop_words;

  await typeText('typesetting ');
  if (stopWordsRemoved) {
    await waitNoResults();
  } else {
    await assertSingle('typesetting');
  }

  // Not a stop word
  await typeText('+npm +run +dev +installmdbook');
  await assertSingle('use the npm run dev script');
  // ------------------------------------------------------

  // ------------------------------------------------------
  // max_term_len test

  const length71Word = 'thisisaverylongnonexistentwordoflength71madetotestthemaxtermlenoptionnn';
  await typeText(length71Word);
  const maxTermLen = sourceConfigFile.lang_config.options.ignore_stop_words;
  if (maxTermLen) {
    await waitNoResults();
  } else {
    await assertSingle(length71Word);
  }

  const length91Word =
    'thisisaverylongnonexistentwordoflength91madetotestthemaxtermlenoptionnnmadetotestmadetotest';
  await typeText(length91Word);
  await waitNoResults();

  // ------------------------------------------------------
}

const cleanup = () => {
  const notFoundFile = path.join(__dirname, 'input/404.html');
  if (fs.existsSync(notFoundFile)) {
    fs.rmSync(notFoundFile);
  }

  const contributingFile = path.join(__dirname, 'input/contributing.html');
  if (fs.existsSync(contributingFile)) {
    fs.rmSync(contributingFile);
  }

  cleanupAddFilesTests();

  replaceIntroductionMood('delightful');
  replaceGettingStartedOthersDate('2022 Dec 11 12:09 +0000');
};

function readOutputConfig() {
  return JSON.parse(
    fs.readFileSync(path.join(__dirname, 'output/output_config.json'), 'utf8'),
  );
}

const mainTest = async () => {
  runFullIndex('e2e/input/infi_search_empty.json');

  cleanup();
  console.log('Starting infi_search_0 tests');
  const config0 = 'e2e/input/infi_search_0.json';
  await testSuite(config0, true, true);

  // Assert what's cached
  // Slightly different pl_cache_thresholds for the 4 tests
  let outputConfig = readOutputConfig();
  expect(outputConfig.indexingConfig.plNamesToCache).toHaveLength(5);
  expect(outputConfig.indexingConfig.plNamesToCache).toEqual([0, 1, 2, 3, 4]);

  // ignore_stop_words=false + "stop_words": ["typesetting"] = results still show
  await testTokenizerOptions(config0);

  cleanup();
  console.log('Starting infi_search_1 tests');
  const config1 = 'e2e/input/infi_search_1.json';
  await testSuite(config1, true, false);

  outputConfig = readOutputConfig();
  expect(outputConfig.indexingConfig.plNamesToCache).toHaveLength(2);
  expect(outputConfig.indexingConfig.plNamesToCache).toEqual([0, 1]);

  // ignore_stop_words=false + default stop words = results still show
  await testTokenizerOptions(config1);

  cleanup();
  console.log('Starting infi_search_2 tests');
  const config2 = 'e2e/input/infi_search_2.json';
  await testSuite(config2, true, false);

  outputConfig = readOutputConfig();
  expect(outputConfig.indexingConfig.plNamesToCache).toHaveLength(2);
  expect(outputConfig.indexingConfig.plNamesToCache).toEqual([0, 1]);

  cleanup();
  console.log('Starting infi_search_3 tests');
  const config3 = 'e2e/input/infi_search_3.json';
  await testSuite(config3, true, false);

  outputConfig = readOutputConfig();
  expect(outputConfig.indexingConfig.plNamesToCache).toHaveLength(0);

  // Latin tokenizer
  // No positions
  cleanup();
  console.log('Starting infi_search_4 tests');
  const config4 = 'e2e/input/infi_search_4.json';
  await testSuite(config4, false, false);

  outputConfig = readOutputConfig();
  expect(outputConfig.indexingConfig.plNamesToCache).toHaveLength(0);

  // Chinese tokenizer
  cleanup();
  console.log('Starting infi_search_5 tests');
  const config5 = 'e2e/input/infi_search_5.json';
  await testSuite(config5, true, false);

  outputConfig = readOutputConfig();
  expect(outputConfig.indexingConfig.plNamesToCache).toHaveLength(4);

  await testChinese();

  // ignore_stop_words = true, max_term_len=70
  cleanup();
  console.log('Starting infi_search_tokenizer tests');
  const configTokenizer = 'e2e/input/infi_search_tokenizer.json';
  await testTokenizerOptions(configTokenizer);
};

test('Single test', async () => {
  try {
    await mainTest();

    cleanup();
    process.exit(0);
  } catch (ex) {
    console.error(ex);
    cleanup();
    process.exit(1);
  }
});
