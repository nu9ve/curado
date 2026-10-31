import { expect, test } from 'bun:test';
import { createElement } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { UnavailableControl } from '../components/UnavailableControl';
import { setLanguage } from './language';

test('unavailable console controls are labelled and cannot dispatch an audio action', () => {
  setLanguage('es');
  const markup = renderToStaticMarkup(
    createElement(UnavailableControl, { label: 'FILTER', dial: true }),
  );
  expect(markup).toContain('aria-disabled="true"');
  expect(markup).toContain('FILTER · próximamente, sin función');
  expect(markup).not.toContain('data-action');
  expect(markup).not.toContain('disabled=""');
  setLanguage('en');
  expect(renderToStaticMarkup(createElement(UnavailableControl, { label: 'CUE' }))).toContain(
    'CUE · coming soon, unavailable',
  );
  setLanguage('es');
});
