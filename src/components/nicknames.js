const adjectives = [
  'happy', 'clever', 'swift', 'brave', 'calm', 'eager', 'gentle', 'jolly',
  'kind', 'lively', 'merry', 'nimble', 'playful', 'quiet', 'reliable',
  'smart', 'tricky', 'vivid', 'witty', 'zippy', 'dynamic', 'frisky', 'quirky'
];

const animals = [
  'cappy', 'dolphin', 'eagle', 'falcon', 'gazelle', 'hare', 'ibis', 'jackal',
  'koala', 'lemur', 'meerkat', 'narwhal', 'otter', 'panda', 'quail', 'raven',
  'salmon', 'tiger', 'urchin', 'viper', 'whale', 'xenops', 'yak', 'zebra'
];

let usedCombinations = new Set();

export function generateDefaultNickname() {
  let combination;
  let attempts = 0;

  do {
    const adj = adjectives[Math.floor(Math.random() * adjectives.length)];
    const animal = animals[Math.floor(Math.random() * animals.length)];
    combination = `${adj}-${animal}`;
    attempts++;
  } while (usedCombinations.has(combination) && attempts < 100);

  if (attempts < 100) {
    usedCombinations.add(combination);
  }

  return combination;
}