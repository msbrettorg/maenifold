import { defaultStyles, mergeStyles, vanillaRenderers } from '@jsonforms/vue-vanilla';

export const themedStyles = Object.freeze(mergeStyles(defaultStyles, {
  control: {
    root: 'schema-form__control',
    wrapper: 'schema-form__control-wrapper',
    label: 'schema-form__label',
    description: 'schema-form__description',
    input: 'field-input',
    error: 'schema-form__error',
    textarea: 'field-textarea',
    select: 'field-select',
    option: 'schema-form__option',
    asterisk: 'schema-form__asterisk',
    required: 'schema-form__required',
  },
  verticalLayout: {
    root: 'schema-form__vertical-layout',
    item: 'schema-form__layout-item',
  },
  horizontalLayout: {
    root: 'schema-form__horizontal-layout',
    item: 'schema-form__layout-item',
  },
  group: {
    root: 'panel schema-form__group',
    label: 'schema-form__group-label',
    item: 'schema-form__group-item',
  },
  arrayList: {
    root: 'schema-form__array',
    legend: 'schema-form__array-legend',
    addButton: 'btn btn-primary schema-form__array-button',
    label: 'schema-form__array-label',
    itemWrapper: 'panel schema-form__array-item-wrapper',
    noData: 'schema-form__empty',
    item: 'schema-form__array-item',
    itemToolbar: 'schema-form__array-toolbar',
    itemLabel: 'schema-form__array-item-label',
    itemContent: 'schema-form__array-item-content',
    itemExpanded: 'schema-form__array-item-expanded',
    itemMoveUp: 'btn btn-ghost schema-form__array-button',
    itemMoveDown: 'btn btn-ghost schema-form__array-button',
    itemDelete: 'btn btn-outline-danger schema-form__array-button',
  },
  label: {
    root: 'schema-form__static-label',
  },
  dialog: {
    root: 'panel schema-form__dialog',
    title: 'schema-form__dialog-title',
    body: 'schema-form__dialog-body',
    actions: 'schema-form__dialog-actions',
    buttonPrimary: 'btn btn-primary schema-form__dialog-button',
    buttonSecondary: 'btn btn-ghost schema-form__dialog-button',
  },
  oneOf: {
    root: 'schema-form__one-of',
  },
  categorization: {
    root: 'schema-form__categorization',
    category: 'btn btn-ghost schema-form__category',
    selected: 'schema-form__category--selected',
    panel: 'schema-form__categorization-panel',
    stepper: 'schema-form__stepper',
    stepperBadge: 'schema-form__stepper-badge',
    stepperLine: 'schema-form__stepper-line',
    stepperFooter: 'schema-form__stepper-footer',
    stepperButtonBack: 'btn btn-ghost schema-form__stepper-button',
    stepperButtonNext: 'btn btn-primary schema-form__stepper-button',
  },
}));

export const frozenRenderers = Object.freeze(vanillaRenderers);
