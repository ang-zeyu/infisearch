const fs = require('fs');
const path = require('path');

const {
  typePhraseOrAnd,
  typeText,
  waitNoResults,
  assertSingle,
  assertMultiple,
  expectNumDeletedDocs,
  reloadPage,
  runIncrementalIndex,
} = require('./utils');

async function addFilesTest(with_positions, configFile) {
  await typePhraseOrAnd('secondary file test from html', with_positions);
  await assertMultiple(['secondary file test from html'], 2);
  
  await typePhraseOrAnd('secondary file test from json', with_positions);
  await assertSingle('secondary file test from json');
  
  // Recursive linkage tests
  await typePhraseOrAnd('recursive main', with_positions);
  await assertSingle('main document');
  
  await typeText(
    '"1st document in the recursive linkage"'
      + ' AND "2nd document in the recursive linkage"'
      + ' AND "3rd document in the recursive linkage"',
  );
  await assertSingle('recursive linkage');
  
  // ------------------------------------------------------
  // Test incremental indexing
  // Addition of previously missing recursively linked file
  // But interpreted as an "update" to the original document
  expectNumDeletedDocs(0);
  
  await typePhraseOrAnd('last document in the recursive linkage', with_positions);
  await waitNoResults();
  
  const lastLinkedFilePath = path.join(__dirname, 'input/add_files_test/recursive/secondary_last.csv');
  fs.copyFileSync(
    path.join(__dirname, 'incremental_indexing/add_files_test/recursive/secondary_last.csv'),
    lastLinkedFilePath,
  );
  
  runIncrementalIndex(configFile);
  
  await reloadPage();
  await typePhraseOrAnd('last document in the recursive linkage', with_positions);
  await assertSingle('last document in the recursive linkage');
  
  expectNumDeletedDocs(1); // update
  
  runIncrementalIndex(configFile);
  expectNumDeletedDocs(1); // stays the same
  
  
  
  
  // Edit
  // update a linked document
  const lastLinkedFileUpdatedContents = fs
    .readFileSync(lastLinkedFilePath, 'utf-8')
    .replace(
      'last document in the recursive linkage',
      'last updated document in the recursive linkage',
    );
  fs.writeFileSync(lastLinkedFilePath, lastLinkedFileUpdatedContents);
  runIncrementalIndex(configFile);
  
  expectNumDeletedDocs(2); // update
  
  await reloadPage();
  await typePhraseOrAnd('last document in the recursive linkage', with_positions);
  await waitNoResults();
  
  await typePhraseOrAnd('last updated document in the recursive linkage', with_positions);
  await assertSingle('last updated document in the recursive linkage');
  
  // update the main document
  const mainDocumentPath = path.join(__dirname, 'input/add_files_test/recursive/main.csv');
  const updatedMainContents = fs
    .readFileSync(mainDocumentPath, 'utf-8')
    .replace(
      'main document in the recursive linkage',
      'main updated document in the recursive linkage',
    );
  fs.writeFileSync(mainDocumentPath, updatedMainContents);
  runIncrementalIndex(configFile);
  
  expectNumDeletedDocs(3); // update
  
  await reloadPage();
  await typePhraseOrAnd('main document in the recursive linkage', with_positions);
  await waitNoResults();
  
  await typePhraseOrAnd('main updated document in the recursive linkage', with_positions);
  await assertSingle('main updated document in the recursive linkage');
  
  runIncrementalIndex(configFile);
  expectNumDeletedDocs(3); // stays the same
  
  
  // Delete the linked document
  fs.rmSync(lastLinkedFilePath);
  
  runIncrementalIndex(configFile);
  expectNumDeletedDocs(4); // deletion
  
  await reloadPage();
  await typePhraseOrAnd('last updated document in the recursive linkage', with_positions);
  await waitNoResults();
  
  runIncrementalIndex(configFile);
  expectNumDeletedDocs(5); // false-positive, forced update (unable to get deleted linked file metadata)
  
  
  // Delete the main document
  fs.rmSync(mainDocumentPath);
  
  runIncrementalIndex(configFile);
  expectNumDeletedDocs(6); // deletion
  
  runIncrementalIndex(configFile);
  expectNumDeletedDocs(6); // stays the same
}

function cleanupAddFilesTests() {
  const mainRecursiveFileSource = path.join(
    __dirname,
    'incremental_indexing/add_files_test/recursive/main.csv',
  );
  const mainRecursiveFile = path.join(__dirname, 'input/add_files_test/recursive/main.csv');
  fs.copyFileSync(mainRecursiveFileSource, mainRecursiveFile);
  
  const secondaryLastFile = path.join(__dirname, 'input/add_files_test/recursive/secondary_last.csv');
  if (fs.existsSync(secondaryLastFile)) {
    fs.rmSync(secondaryLastFile);
  }
}

module.exports = {
  addFilesTest,
  cleanupAddFilesTests,
};
