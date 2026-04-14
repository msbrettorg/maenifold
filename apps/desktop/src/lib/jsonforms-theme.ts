import { defaultStyles, mergeStyles, vanillaRenderers } from '@jsonforms/vue-vanilla';

const controlClass =
  'w-full rounded-md border border-border bg-input px-3 py-2 text-caption text-foreground outline-none focus:border-ring focus:ring-2 focus:ring-ring/20';
const textareaClass = `${controlClass} min-h-20 resize-y`;
const panelClass = 'rounded-lg border border-border bg-card p-4';
const buttonBaseClass =
  'inline-flex min-h-9 items-center justify-center gap-2 whitespace-nowrap rounded-md px-4 py-2 text-caption font-medium transition-colors disabled:pointer-events-none disabled:opacity-50';
const primaryButtonClass = `${buttonBaseClass} bg-primary text-primary-foreground hover:bg-primary/90`;
const ghostButtonClass = `${buttonBaseClass} border border-transparent bg-transparent text-muted-foreground hover:bg-muted hover:text-foreground`;
const destructiveButtonClass = `${buttonBaseClass} border border-destructive/70 bg-transparent text-destructive hover:bg-destructive/10`;

export const themedStyles = Object.freeze(mergeStyles(defaultStyles, {
  control: {
    root: 'schema-form__control',
    wrapper: 'schema-form__control-wrapper',
    label: 'schema-form__label',
    description: 'schema-form__description',
    input: controlClass,
    error: 'schema-form__error',
    textarea: textareaClass,
    select: controlClass,
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
    root: `${panelClass} schema-form__group`,
    label: 'schema-form__group-label',
    item: 'schema-form__group-item',
  },
  arrayList: {
    root: 'schema-form__array',
    legend: 'schema-form__array-legend',
    addButton: `${primaryButtonClass} schema-form__array-button`,
    label: 'schema-form__array-label',
    itemWrapper: `${panelClass} schema-form__array-item-wrapper`,
    noData: 'schema-form__empty',
    item: 'schema-form__array-item',
    itemToolbar: 'schema-form__array-toolbar',
    itemLabel: 'schema-form__array-item-label',
    itemContent: 'schema-form__array-item-content',
    itemExpanded: 'schema-form__array-item-expanded',
    itemMoveUp: `${ghostButtonClass} schema-form__array-button`,
    itemMoveDown: `${ghostButtonClass} schema-form__array-button`,
    itemDelete: `${destructiveButtonClass} schema-form__array-button`,
  },
  label: {
    root: 'schema-form__static-label',
  },
  dialog: {
    root: `${panelClass} schema-form__dialog`,
    title: 'schema-form__dialog-title',
    body: 'schema-form__dialog-body',
    actions: 'schema-form__dialog-actions',
    buttonPrimary: `${primaryButtonClass} schema-form__dialog-button`,
    buttonSecondary: `${ghostButtonClass} schema-form__dialog-button`,
  },
  oneOf: {
    root: 'schema-form__one-of',
  },
  categorization: {
    root: 'schema-form__categorization',
    category: `${ghostButtonClass} schema-form__category`,
    selected: 'schema-form__category--selected',
    panel: 'schema-form__categorization-panel',
    stepper: 'schema-form__stepper',
    stepperBadge: 'schema-form__stepper-badge',
    stepperLine: 'schema-form__stepper-line',
    stepperFooter: 'schema-form__stepper-footer',
    stepperButtonBack: `${ghostButtonClass} schema-form__stepper-button`,
    stepperButtonNext: `${primaryButtonClass} schema-form__stepper-button`,
  },
}));

export const frozenRenderers = Object.freeze(vanillaRenderers);
