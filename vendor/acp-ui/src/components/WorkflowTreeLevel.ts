import { defineComponent, h, type PropType, type VNode } from 'vue';
import type { WorkflowTreeNode as WTN } from '../lib/workflow-types';
import WorkflowNodeCard from './WorkflowNodeCard.vue';
import ThinkingSessionCard from './ThinkingSessionCard.vue';

/**
 * WorkflowTreeLevel — recursively renders one level of the workflow tree.
 *
 * Layout per node:
 *   [WorkflowNodeCard or ThinkingSessionCard]
 *       │  (connector line)
 *   [children row with bracket connectors]
 *       recursion…
 */
export const WorkflowTreeLevel = defineComponent({
  name: 'WorkflowTreeLevel',

  props: {
    node: {
      type: Object as PropType<WTN>,
      required: true,
    },
    depth: {
      type: Number,
      default: 0,
    },
  },

  setup(props) {
    const CONN_WORKFLOW = '#22c55e';
    const CONN_THINKING = '#8b5cf6';

    function hasChildren(node: WTN): boolean {
      return node.children.length > 0;
    }

    function childColor(child: WTN): string {
      return child.type === 'thinking' ? CONN_THINKING : CONN_WORKFLOW;
    }

    /** Render the card for this node */
    function renderCard(node: WTN): VNode {
      if (node.type === 'workflow') {
        return h(WorkflowNodeCard, { node: node.data, key: node.data.uri });
      }
      return h(ThinkingSessionCard, { node: node.data, key: node.data.uri });
    }

    /**
     * Build SVG connector bracket for a list of children.
     *
     * For a single child: simple vertical drop
     *   ┃
     *   ●
     *
     * For multiple children: T-bracket
     *   ┃
     *   ┣━━━━┳━━━━┓
     *   ●    ●    ●
     */
    function renderConnector(children: WTN[]): VNode | null {
      if (children.length === 0) return null;

      const svgH = 40;          // connector SVG height in px
      const nodeW = 300;        // approximate card width
      const gap = 24;           // gap between cards

      if (children.length === 1) {
        const col = childColor(children[0]);
        return h(
          'svg',
          {
            class: 'tree-connector single-connector',
            viewBox: `0 0 ${nodeW} ${svgH}`,
            width: nodeW,
            height: svgH,
            'aria-hidden': 'true',
          },
          [
            h('line', {
              x1: nodeW / 2, y1: 0,
              x2: nodeW / 2, y2: svgH,
              stroke: col,
              'stroke-width': 2,
              'stroke-dasharray': '4 3',
            }),
          ]
        );
      }

      // Multiple children — calculate positions
      const totalW = children.length * nodeW + (children.length - 1) * gap;
      const midX = totalW / 2;  // parent card is centred over the full children band
      const startX = 0;

      const childMidXs = children.map((_, i) => startX + i * (nodeW + gap) + nodeW / 2);
      const bracketY = svgH * 0.65;

      const svgChildren: VNode[] = [
        // vertical drop from parent to bracket bar
        h('line', {
          x1: midX, y1: 0,
          x2: midX, y2: bracketY,
          stroke: '#4b5563',
          'stroke-width': 2,
          'stroke-dasharray': '4 3',
        }),
        // horizontal bracket bar
        h('line', {
          x1: childMidXs[0], y1: bracketY,
          x2: childMidXs[childMidXs.length - 1], y2: bracketY,
          stroke: '#4b5563',
          'stroke-width': 2,
        }),
        // vertical drops per child with colour
        ...children.flatMap((child, i) => {
          const cx = childMidXs[i];
          const col = childColor(child);
          return [
            h('line', {
              key: `drop-${i}`,
              x1: cx, y1: bracketY,
              x2: cx, y2: svgH,
              stroke: col,
              'stroke-width': 2,
              'stroke-dasharray': '4 3',
            }),
            // coloured dot at bottom
            h('circle', {
              key: `dot-${i}`,
              cx, cy: svgH - 3,
              r: 3,
              fill: col,
            }),
          ];
        }),
      ];

      return h(
        'svg',
        {
          class: 'tree-connector multi-connector',
          viewBox: `0 0 ${totalW} ${svgH}`,
          width: totalW,
          height: svgH,
          'aria-hidden': 'true',
          style: { alignSelf: 'flex-start' },
        },
        svgChildren,
      );
    }

    /** Render children row */
    function renderChildrenRow(children: WTN[], depth: number): VNode {
      return h(
        'div',
        { class: 'children-row' },
        children.map((child) =>
          h(WorkflowTreeLevel, {
            node: child,
            depth: depth + 1,
            key: child.data.uri,
          })
        )
      );
    }

    return () => {
      const node = props.node;
      const depth = props.depth;
      const children = node.children;
      const showChildren = hasChildren(node);

      return h(
        'div',
        {
          class: [
            'tree-level',
            `depth-${depth}`,
            node.type === 'thinking' ? 'level-thinking' : 'level-workflow',
          ],
        },
        [
          renderCard(node),
          showChildren
            ? h('div', { class: 'connector-wrapper' }, [
                renderConnector(children),
              ])
            : null,
          showChildren ? renderChildrenRow(children, depth) : null,
        ].filter(Boolean) as VNode[]
      );
    };
  },
});
