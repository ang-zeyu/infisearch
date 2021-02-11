function indexer() {
  if (process.env.x === '1') {
    return 1;
  }
  return 0;
}

module.exports = indexer;
