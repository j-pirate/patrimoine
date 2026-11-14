/* Executed by WebDriver in the real WebKit window, never shipped in the app. */
window.auditPatrimoineAccessibility = async function () {
  if (!window.axe) throw new Error('Le moteur axe-core est absent de la fenêtre.');
  const result = await window.axe.run(document, {
    runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'] },
  });
  return {
    engine: result.testEngine,
    environment: result.testEnvironment,
    url: result.url,
    timestamp: result.timestamp,
    passes_count: result.passes.length,
    inapplicable_count: result.inapplicable.length,
    violations: result.violations,
    incomplete: result.incomplete,
  };
};
