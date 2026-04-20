# Maenifold Desktop App - User Acceptance Test Plan

**Version:** 1.0  
**Date:** 2026-04-19  
**Status:** Draft  
**Total Test Cases:** 575

---

## Overview

This document provides an exhaustive User Acceptance Test (UAT) plan for the Maenifold desktop application. The app runs in two contexts:
- **Electron**: Standalone desktop application with full native capabilities
- **Cowork iframe**: Embedded within the Cowork host application with restricted capabilities

Test cases are organized by screen/feature area and cover every interactive element identified in the component audit.

### Test Case Format

| Column | Description |
|--------|-------------|
| TC-ID | Sequential identifier (TC-001, TC-002, etc.) |
| Screen | Which screen/tab/dialog contains the element |
| Component | Specific Vue component name |
| Element | The interactive element being tested |
| Action | What the tester performs |
| Expected Result | Observable outcome |
| Context | Electron \| Cowork iframe \| Both |

### Legend

- ⚠️ = Known iframe limitation
- 🔴 = Destructive action requiring confirmation
- 🔑 = Requires authentication
- 📋 = Clipboard operation

---

## A. Application Shell

### A.1 Tab Navigation (AppTabBar)

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-001 | Shell | AppTabBar | Home tab | Click tab | Home dashboard displays, tab shows active state | Both |
| TC-002 | Shell | AppTabBar | Home tab | Press Enter when focused | Home dashboard displays | Both |
| TC-003 | Shell | AppTabBar | Agents tab | Click tab | Agents view displays, tab shows active state | Both |
| TC-004 | Shell | AppTabBar | Pipelines tab | Click tab | Pipeline view displays, tab shows active state | Both |
| TC-005 | Shell | AppTabBar | Gate tab | Click tab | Gate view displays, tab shows active state | Both |
| TC-006 | Shell | AppTabBar | Knowledge tab | Click tab | Knowledge explorer displays, tab shows active state | Both |
| TC-007 | Shell | AppTabBar | Workflows tab | Click tab | Workflow authoring page displays, tab shows active state | Both |
| TC-008 | Shell | AppTabBar | Settings tab | Click tab | Settings view displays, tab shows active state | Both |
| TC-009 | Shell | AppTabBar | Tab bar | Press Right Arrow on focused tab | Focus moves to next tab | Both |
| TC-010 | Shell | AppTabBar | Tab bar | Press Left Arrow on focused tab | Focus moves to previous tab | Both |
| TC-011 | Shell | AppTabBar | Tab bar | Press Home key | Focus moves to first tab | Both |
| TC-012 | Shell | AppTabBar | Tab bar | Press End key | Focus moves to last tab | Both |
| TC-013 | Shell | AppTabBar | Tab bar | Tab through tabs | Focus order is logical (left to right) | Both |
| TC-014 | Shell | AppTabBar | Active tab indicator | Observe after tab switch | Visual indicator shows current tab | Both |
| TC-015 | Shell | AppTabBar | Tab icons | Observe all tabs | Each tab has distinct recognizable icon | Both |
| TC-016 | Shell | AppTabBar | Tab labels | Observe all tabs | Labels are readable, not truncated | Both |

### A.2 Sidebar (AppSidebar)

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-017 | Shell | AppSidebar | Traffic monitor toggle | Click toggle | Traffic monitor panel opens/closes | Both |
| TC-018 | Shell | AppSidebar | Collapse button | Click button | Sidebar collapses to icon-only mode | Both |
| TC-019 | Shell | AppSidebar | Expand button | Click when collapsed | Sidebar expands to full width | Both |
| TC-020 | Shell | AppSidebar | Folder select (📁) | Click button | Folder picker dialog opens | Electron |
| TC-021 | Shell | AppSidebar | Folder select (📁) | Click button | Shows error or fallback (folder picker broken in iframe) | Cowork iframe ⚠️ |
| TC-022 | Shell | AppSidebar | New session (+) | Click button | New chat session is created | Both |
| TC-023 | Shell | AppSidebar | New session (+) | Press Enter when focused | New chat session is created | Both |
| TC-024 | Shell | AppSidebar | AgentSelector | Click selector | Dropdown of agents appears | Both |
| TC-025 | Shell | AppSidebar | AgentSelector | Select agent from list | Agent changes, session updates | Both |
| TC-026 | Shell | AppSidebar | AgentSelector | Press Escape after opening | Dropdown closes | Both |
| TC-027 | Shell | AppSidebar | SessionList | Observe with sessions | List shows all sessions | Both |
| TC-028 | Shell | AppSidebar | SessionList | Observe with no sessions | Empty state message displays | Both |
| TC-029 | Shell | AppSidebar | ConnectionBanner | Observe when disconnected | Banner shows connection status | Both |
| TC-030 | Shell | AppSidebar | ConnectionBanner cancel | Click cancel button | Reconnection attempt cancelled | Both |
| TC-031 | Shell | AppSidebar | Resize handle | Drag handle | Sidebar width changes | Both |
| TC-032 | Shell | AppSidebar | Resize handle | Drag to minimum | Sidebar snaps to minimum width | Both |
| TC-033 | Shell | AppSidebar | Resize handle | Drag to maximum | Sidebar snaps to maximum width | Both |
| TC-034 | Shell | AppSidebar | Sidebar | Observe z-index | Sidebar overlays content properly, no clipping | Both |

### A.3 Home Dashboard (HomeDashboard)

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-035 | Home | HomeDashboard | Category tile 1 | Click tile | Navigates to appropriate section | Both |
| TC-036 | Home | HomeDashboard | Category tile 2 | Click tile | Navigates to appropriate section | Both |
| TC-037 | Home | HomeDashboard | Category tile 3 | Click tile | Navigates to appropriate section | Both |
| TC-038 | Home | HomeDashboard | Category tile 4 | Click tile | Navigates to appropriate section | Both |
| TC-039 | Home | HomeDashboard | View all agents button | Click button | Switches to Agents tab | Both |
| TC-040 | Home | HomeDashboard | Quick input field | Type text | Text appears in input | Both |
| TC-041 | Home | HomeDashboard | Quick input field | Paste text | Pasted text appears | Both |
| TC-042 | Home | HomeDashboard | Quick input field | Submit empty | Validation prevents submission | Both |
| TC-043 | Home | HomeDashboard | Quick input field | Type max-length text | Input accepts or truncates appropriately | Both |
| TC-044 | Home | HomeDashboard | Send button | Click with text | Message sent, navigates to chat | Both |
| TC-045 | Home | HomeDashboard | Send button | Click when empty | Button disabled or shows validation | Both |
| TC-046 | Home | HomeDashboard | Send button | Press Enter in input | Same as clicking Send | Both |
| TC-047 | Home | HomeDashboard | Dashboard | Observe loading state | Loading indicator displays during fetch | Both |
| TC-048 | Home | HomeDashboard | Dashboard | Observe error state | Error message displays on failure | Both |

### A.4 Empty State

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-049 | Various | EmptyState | Action button | Click button | Appropriate action triggered | Both |
| TC-050 | Various | EmptyState | Action button | Press Enter when focused | Appropriate action triggered | Both |
| TC-051 | Various | EmptyState | Container | Observe | Centered, appropriate spacing, readable text | Both |

### A.5 Command Palette

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-052 | Shell | CommandPalette | Trigger | Press Cmd+K (Mac) / Ctrl+K (Win) | Command palette opens | Both |
| TC-053 | Shell | CommandPalette | Search input | Type text | Commands filter in real-time | Both |
| TC-054 | Shell | CommandPalette | Search input | Clear input | All commands show again | Both |
| TC-055 | Shell | CommandPalette | Command item | Click item | Command executes, palette closes | Both |
| TC-056 | Shell | CommandPalette | Command item | Press Enter on focused item | Command executes, palette closes | Both |
| TC-057 | Shell | CommandPalette | Command list | Press Down Arrow | Focus moves to next command | Both |
| TC-058 | Shell | CommandPalette | Command list | Press Up Arrow | Focus moves to previous command | Both |
| TC-059 | Shell | CommandPalette | Palette | Press Escape | Palette closes | Both |
| TC-060 | Shell | CommandPalette | Palette | Click outside | Palette closes | Both |
| TC-061 | Shell | CommandPalette | Palette | Observe z-index | Palette appears above all content | Both |
| TC-062 | Shell | CommandPalette | Search input | Paste text | Pasted text filters commands | Both |
| TC-063 | Shell | CommandPalette | No results | Search for gibberish | "No commands found" message | Both |

### A.6 Toast Container

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-064 | Shell | ToastContainer | Toast | Observe auto-dismiss | Toast disappears after timeout | Both |
| TC-065 | Shell | ToastContainer | Dismiss button | Click X button | Toast immediately dismissed | Both |
| TC-066 | Shell | ToastContainer | Multiple toasts | Trigger several toasts | Toasts stack properly without overlap | Both |
| TC-067 | Shell | ToastContainer | Toast position | Observe | Toasts appear in correct corner | Both |
| TC-068 | Shell | ToastContainer | Success toast | Trigger success | Green/success styling applied | Both |
| TC-069 | Shell | ToastContainer | Error toast | Trigger error | Red/error styling applied | Both |
| TC-070 | Shell | ToastContainer | Warning toast | Trigger warning | Yellow/warning styling applied | Both |
| TC-071 | Shell | ToastContainer | Info toast | Trigger info | Blue/info styling applied | Both |

### A.7 Panel Resize Handle

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-072 | Shell | PanelResizeHandle | Handle | Hover | Cursor changes to resize cursor | Both |
| TC-073 | Shell | PanelResizeHandle | Handle | Drag left | Left panel shrinks, right expands | Both |
| TC-074 | Shell | PanelResizeHandle | Handle | Drag right | Left panel expands, right shrinks | Both |
| TC-075 | Shell | PanelResizeHandle | Handle | Double-click | Panels reset to default proportions | Both |

---

## B. Permissions & Auth Dialogs

### B.1 Permission Dialog

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-076 | Dialog | PermissionDialog | Dialog | Observe when triggered | Modal appears with permission request | Both |
| TC-077 | Dialog | PermissionDialog | Permission list | Observe | All requested permissions displayed | Both |
| TC-078 | Dialog | PermissionDialog | Select button | Click for a permission | Permission granted, item updates | Both |
| TC-079 | Dialog | PermissionDialog | Cancel button | Click | Dialog closes, permission denied | Both |
| TC-080 | Dialog | PermissionDialog | Dialog | Press Escape | Dialog closes | Both |
| TC-081 | Dialog | PermissionDialog | Dialog backdrop | Click outside | Dialog closes (or stays open per design) | Both |

### B.2 Permission Entry

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-082 | Dialog | PermissionEntry | Option button 1 | Click | Option selected, visual feedback | Both |
| TC-083 | Dialog | PermissionEntry | Option button 2 | Click | Option selected, visual feedback | Both |
| TC-084 | Dialog | PermissionEntry | Cancel | Click per-entry cancel | Request cancelled for this entry | Both |
| TC-085 | Dialog | PermissionEntry | Entry | Observe | Permission name and scope are readable | Both |

### B.3 Auth Method Dialog

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-086 | Dialog | AuthMethodDialog | Close X button | Click | Dialog closes | Both |
| TC-087 | Dialog | AuthMethodDialog | Method button 1 | Click | Auth method selected, flow continues | Both |
| TC-088 | Dialog | AuthMethodDialog | Method button 2 | Click | Auth method selected, flow continues | Both |
| TC-089 | Dialog | AuthMethodDialog | Cancel button | Click | Dialog closes, auth cancelled | Both |
| TC-090 | Dialog | AuthMethodDialog | Dialog | Press Escape | Dialog closes | Both |
| TC-091 | Dialog | AuthMethodDialog | Method buttons | Tab through | Focus moves through options logically | Both |

### B.4 Tool Auth Setup Dialog

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-092 | Dialog | ToolAuthSetupDialog | Cancel button | Click | Dialog closes, setup cancelled | Both |
| TC-093 | Dialog | ToolAuthSetupDialog | Confirm button | Click with valid data | Auth configured, dialog closes | Both |
| TC-094 | Dialog | ToolAuthSetupDialog | Confirm button | Click with invalid data | Validation errors shown | Both |
| TC-095 | Dialog | ToolAuthSetupDialog | Dialog | Press Escape | Dialog closes | Both |
| TC-096 | Dialog | ToolAuthSetupDialog | Form fields | Tab through | Focus order is logical | Both |

### B.5 Folder Picker

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-097 | Dialog | folder-picker | Path input | Type path | Path text appears | Electron |
| TC-098 | Dialog | folder-picker | Path input | Paste path | Pasted path appears | Electron |
| TC-099 | Dialog | folder-picker | Cancel button | Click | Picker closes, no selection | Electron |
| TC-100 | Dialog | folder-picker | Use Path button | Click with valid path | Path selected, picker closes | Electron |
| TC-101 | Dialog | folder-picker | Use Path button | Click with invalid path | Validation error shown | Electron |
| TC-102 | Dialog | folder-picker | Picker | Attempt to open | Graceful failure message (BROKEN) | Cowork iframe ⚠️ |

---

## C. Chat Surface

### C.1 Chat View

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-103 | Chat | ChatView | Container | Observe empty state | "Start a conversation" prompt shown | Both |
| TC-104 | Chat | ChatView | Container | Observe with messages | Messages render in correct order | Both |
| TC-105 | Chat | ChatView | Container | Observe during loading | Loading indicator visible | Both |
| TC-106 | Chat | ChatView | Container | Observe error state | Error message with retry option | Both |
| TC-107 | Chat | ChatView | ConfigOptionPicker | Click if present | Config options shown | Both |

### C.2 Chat Input Bar

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-108 | Chat | ChatInputBar | Textarea | Type message | Text appears in textarea | Both |
| TC-109 | Chat | ChatInputBar | Textarea | Paste text | Pasted text appears | Both |
| TC-110 | Chat | ChatInputBar | Textarea | Type very long message | Textarea expands or scrolls | Both |
| TC-111 | Chat | ChatInputBar | Textarea | Press Enter | Message sends (single line mode) | Both |
| TC-112 | Chat | ChatInputBar | Textarea | Press Shift+Enter | New line inserted (no send) | Both |
| TC-113 | Chat | ChatInputBar | Textarea | Type empty and press Enter | No message sent | Both |
| TC-114 | Chat | ChatInputBar | Send button | Click with text | Message sent, textarea clears | Both |
| TC-115 | Chat | ChatInputBar | Send button | Click when empty | Button disabled or no action | Both |
| TC-116 | Chat | ChatInputBar | Send button | Observe during send | Button shows sending state | Both |
| TC-117 | Chat | ChatInputBar | Embedded context button | Click | Context selector opens | Both |
| TC-118 | Chat | ChatInputBar | Input bar | Observe z-index | Bar stays at bottom, content scrolls above | Both |

### C.3 Chat Attachment Bar

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-119 | Chat | ChatAttachmentBar | Image trigger button | Click | File picker opens for images | Both |
| TC-120 | Chat | ChatAttachmentBar | Audio trigger button | Click | File picker opens for audio | Both |
| TC-121 | Chat | ChatAttachmentBar | Image file input | Select image file | Image attachment added | Both |
| TC-122 | Chat | ChatAttachmentBar | Audio file input | Select audio file | Audio attachment added | Both |
| TC-123 | Chat | ChatAttachmentBar | Attachment remove button | Click X on attachment | Attachment removed | Both |
| TC-124 | Chat | ChatAttachmentBar | Multiple attachments | Add 3+ attachments | All display correctly | Both |
| TC-125 | Chat | ChatAttachmentBar | Large file | Select oversized file | Validation error shown | Both |
| TC-126 | Chat | ChatAttachmentBar | Invalid file type | Select .exe file | Validation error shown | Both |

### C.4 Chat Message List

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-127 | Chat | ChatMessageList | Suggestion chip 1 | Click | Suggestion text sent as message | Both |
| TC-128 | Chat | ChatMessageList | Suggestion chip 2 | Click | Suggestion text sent as message | Both |
| TC-129 | Chat | ChatMessageList | Cancel button | Click during streaming | Response cancelled | Both |
| TC-130 | Chat | ChatMessageList | Message list | Scroll up | Older messages visible | Both |
| TC-131 | Chat | ChatMessageList | Message list | New message arrives | Auto-scrolls to bottom | Both |
| TC-132 | Chat | ChatMessageList | Message list | Scrolled up + new message | Does NOT auto-scroll (preserves position) | Both |

### C.5 Chat Message Item

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-133 | Chat | ChatMessageItem | Toggle thought button | Click | Thought section expands/collapses | Both |
| TC-134 | Chat | ChatMessageItem | Toggle tool call button | Click | Tool call section expands/collapses | Both |
| TC-135 | Chat | ChatMessageItem | User message | Observe | Shows user avatar, correct alignment | Both |
| TC-136 | Chat | ChatMessageItem | Assistant message | Observe | Shows assistant avatar, correct alignment | Both |
| TC-137 | Chat | ChatMessageItem | Code block | Observe | Syntax highlighting applied | Both |
| TC-138 | Chat | ChatMessageItem | Markdown | Observe | Headings, lists, links render correctly | Both |

### C.6 Chat Thought Section

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-139 | Chat | ChatThoughtSection | Toggle thinking button | Click when collapsed | Thinking content expands | Both |
| TC-140 | Chat | ChatThoughtSection | Toggle thinking button | Click when expanded | Thinking content collapses | Both |
| TC-141 | Chat | ChatThoughtSection | Content | Observe expanded | Thinking text readable, formatted | Both |

### C.7 Chat Tool Call Item

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-142 | Chat | ChatToolCallItem | Toggle details button | Click when collapsed | Tool details expand | Both |
| TC-143 | Chat | ChatToolCallItem | Toggle details button | Click when expanded | Tool details collapse | Both |
| TC-144 | Chat | ChatToolCallItem | Tool name | Observe | Tool name displayed clearly | Both |
| TC-145 | Chat | ChatToolCallItem | Tool status | Observe running | Shows running indicator | Both |
| TC-146 | Chat | ChatToolCallItem | Tool status | Observe completed | Shows success indicator | Both |
| TC-147 | Chat | ChatToolCallItem | Tool status | Observe failed | Shows error indicator | Both |

### C.8 Session List

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-148 | Sidebar | SessionList | Resume button | Click | Session restored, chat loads | Both |
| TC-149 | Sidebar | SessionList | Delete button | Click | Confirmation prompt appears | Both |
| TC-150 | Sidebar | SessionList | Delete confirm | Click confirm | Session deleted 🔴 | Both |
| TC-151 | Sidebar | SessionList | Delete cancel | Click cancel | Session preserved | Both |
| TC-152 | Sidebar | SessionList | Session item | Observe | Shows title, timestamp | Both |
| TC-153 | Sidebar | SessionList | Session list | Scroll | Older sessions accessible | Both |
| TC-154 | Sidebar | SessionList | Empty state | Observe with no sessions | "No sessions" message | Both |

### C.9 Startup Progress

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-155 | Startup | StartupProgress | Toggle details | Click | Detailed progress shown/hidden | Both |
| TC-156 | Startup | StartupProgress | Cancel button | Click | Startup cancelled | Both |
| TC-157 | Startup | StartupProgress | Progress bar | Observe | Shows accurate progress | Both |
| TC-158 | Startup | StartupProgress | Status text | Observe | Current step described | Both |

---

## D. Pipeline View & Composer

### D.1 Pipeline View

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-159 | Pipeline | PipelineView | Banner dismiss | Click X | Banner dismissed | Both |
| TC-160 | Pipeline | PipelineView | Mobile sidebar toggle | Click | Sidebar toggles (mobile) | Both |
| TC-161 | Pipeline | PipelineView | Flyout toggle | Click | Flyout panel opens/closes | Both |
| TC-162 | Pipeline | PipelineView | Graft button | Click | Graft dialog opens | Both |
| TC-163 | Pipeline | PipelineView | Join button | Click | Join pipeline dialog opens | Both |
| TC-164 | Pipeline | PipelineView | Runner toggle | Click | Runner panel toggles | Both |
| TC-165 | Pipeline | PipelineView | Container | Observe empty state | "Create a pipeline" prompt | Both |

### D.2 Pipeline Sidebar

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-166 | Pipeline | PipelineSidebar | Refresh button | Click | Pipeline list refreshes | Both |
| TC-167 | Pipeline | PipelineSidebar | +New button | Click | New pipeline created | Both |
| TC-168 | Pipeline | PipelineSidebar | Row select | Click row | Pipeline selected, displays in canvas | Both |
| TC-169 | Pipeline | PipelineSidebar | Row delete | Click delete icon | Confirmation shown | Both |
| TC-170 | Pipeline | PipelineSidebar | Delete confirm | Confirm deletion | Pipeline deleted 🔴 | Both |
| TC-171 | Pipeline | PipelineSidebar | Pipeline list | Scroll | All pipelines accessible | Both |

### D.3 Pipeline Flyout

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-172 | Pipeline | PipelineFlyout | Refresh button | Click | Flyout content refreshes | Both |
| TC-173 | Pipeline | PipelineFlyout | Search input | Type text | Items filter | Both |
| TC-174 | Pipeline | PipelineFlyout | Search input | Clear | All items show | Both |
| TC-175 | Pipeline | PipelineFlyout | Retry button | Click | Failed operation retried | Both |
| TC-176 | Pipeline | PipelineFlyout | New pipeline button | Click | New pipeline wizard starts | Both |

### D.4 Pipeline Icon Rail

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-177 | Pipeline | PipelineIconRail | Toggle flyout | Click | Flyout toggles | Both |
| TC-178 | Pipeline | PipelineIconRail | New pipeline | Click | New pipeline created | Both |

### D.5 Pipeline Action Bar

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-179 | Pipeline | PipelineActionBar | Run button | Click when valid | Pipeline execution starts | Both |
| TC-180 | Pipeline | PipelineActionBar | Run button | Observe when invalid | Button disabled | Both |
| TC-181 | Pipeline | PipelineActionBar | Cancel button | Click during run | Execution cancelled | Both |
| TC-182 | Pipeline | PipelineActionBar | Cancel button | Observe when not running | Button disabled/hidden | Both |
| TC-183 | Pipeline | PipelineActionBar | Other action buttons | Click each | Appropriate action triggered | Both |

### D.6 Pipeline Composer (Canvas)

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-184 | Pipeline | PipelineComposer | Canvas | Pan with mouse drag | Canvas pans | Both |
| TC-185 | Pipeline | PipelineComposer | Canvas | Zoom with scroll wheel | Canvas zooms | Both |
| TC-186 | Pipeline | PipelineComposer | Canvas | Pinch to zoom (trackpad) | Canvas zooms | Both |
| TC-187 | Pipeline | PipelineComposer | Node | Click | Node selected | Both |
| TC-188 | Pipeline | PipelineComposer | Node | Drag | Node repositions | Both |
| TC-189 | Pipeline | PipelineComposer | Node | Double-click | Node editor opens | Both |
| TC-190 | Pipeline | PipelineComposer | Edge | Click | Edge selected | Both |
| TC-191 | Pipeline | PipelineComposer | Edge handle | Drag from source | New edge creation starts | Both |
| TC-192 | Pipeline | PipelineComposer | Edge handle | Drop on target | Edge created | Both |
| TC-193 | Pipeline | PipelineComposer | Edge | Delete key when selected | Edge deleted | Both |
| TC-194 | Pipeline | PipelineComposer | Multiple nodes | Shift+click | Multi-selection | Both |
| TC-195 | Pipeline | PipelineComposer | Selection box | Drag to select | Box selection of nodes | Both |
| TC-196 | Pipeline | PipelineComposer | Canvas | Right-click empty area | Context menu (design mode) | Both |
| TC-197 | Pipeline | PipelineComposer | Node | Right-click | Node context menu | Both |

### D.7 Pipeline Composer Toolbar

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-198 | Pipeline | PipelineComposerToolbar | Layout LR button | Click | Nodes arrange left-to-right | Both |
| TC-199 | Pipeline | PipelineComposerToolbar | Layout TB button | Click | Nodes arrange top-to-bottom | Both |
| TC-200 | Pipeline | PipelineComposerToolbar | Delete button | Click with selection | Selected nodes deleted | Both |
| TC-201 | Pipeline | PipelineComposerToolbar | Delete button | Observe with no selection | Button disabled | Both |
| TC-202 | Pipeline | PipelineComposerToolbar | Fit View button | Click | Canvas fits all nodes | Both |
| TC-203 | Pipeline | PipelineComposerToolbar | Toggle Palette button | Click | Node palette toggles | Both |

### D.8 Pipeline Composer Context Menu (Design Mode)

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-204 | Pipeline | ContextMenuDesign | Ask agent | Click | Agent prompt dialog opens | Both |
| TC-205 | Pipeline | ContextMenuDesign | Think through | Click | Sequential thinking node added | Both |
| TC-206 | Pipeline | ContextMenuDesign | Run workflow | Click | Workflow node added | Both |
| TC-207 | Pipeline | ContextMenuDesign | Edit | Click (on node) | Node editor opens | Both |
| TC-208 | Pipeline | ContextMenuDesign | Duplicate | Click (on node) | Node duplicated | Both |
| TC-209 | Pipeline | ContextMenuDesign | Delete | Click (on node) | Node deleted | Both |
| TC-210 | Pipeline | ContextMenuDesign | Disconnect edges | Click (on node) | All edges removed from node | Both |
| TC-211 | Pipeline | ContextMenuDesign | Menu | Press Escape | Menu closes | Both |
| TC-212 | Pipeline | ContextMenuDesign | Menu | Click outside | Menu closes | Both |

### D.9 Pipeline Composer Context Menu (Server Mode)

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-213 | Pipeline | ContextMenuServer | Restart | Click | Node execution restarts | Both |
| TC-214 | Pipeline | ContextMenuServer | Rewind | Click | Node and downstream reset | Both |
| TC-215 | Pipeline | ContextMenuServer | Fork | Click | Pipeline forked from node | Both |
| TC-216 | Pipeline | ContextMenuServer | Abandon | Click | Node marked abandoned | Both |
| TC-217 | Pipeline | ContextMenuServer | Run Pipeline | Click | Pipeline execution starts | Both |
| TC-218 | Pipeline | ContextMenuServer | Graft | Click | Graft dialog opens | Both |
| TC-219 | Pipeline | ContextMenuServer | Promote to Memory | Click | Promotion dialog opens | Both |
| TC-220 | Pipeline | ContextMenuServer | Menu | Press Escape | Menu closes | Both |

### D.10 Pipeline Runner

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-221 | Pipeline | PipelineRunner | Event log | Observe during run | Events stream in real-time | Both |
| TC-222 | Pipeline | PipelineRunner | Event log | Scroll up | Can review past events | Both |
| TC-223 | Pipeline | PipelineRunner | Event log | Observe completed | Final status shown | Both |

### D.11 Pipeline Node Cards

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-224 | Pipeline | PipelineNodeCard | Prompt node | Observe | Correct icon, prompt text visible | Both |
| TC-225 | Pipeline | PipelineNodeCard | SequentialThinking node | Observe | Correct icon, topic visible | Both |
| TC-226 | Pipeline | PipelineNodeCard | Workflow node | Observe | Correct icon, workflow name visible | Both |
| TC-227 | Pipeline | PipelineNodeCard | Tool node | Observe | Correct icon, tool name visible | Both |
| TC-228 | Pipeline | PipelineNodeCard | Node title | Observe | Title not truncated or shows ellipsis | Both |
| TC-229 | Pipeline | PipelineNodeCard | Node | Hover | Hover state visible | Both |
| TC-230 | Pipeline | PipelineNodeCard | Node | Select | Selection border visible | Both |

### D.12 Pipeline Node Card Ports

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-231 | Pipeline | PipelineNodeCardPorts | Target handle (left) | Hover | Handle highlights | Both |
| TC-232 | Pipeline | PipelineNodeCardPorts | Source handle (right) | Hover | Handle highlights | Both |
| TC-233 | Pipeline | PipelineNodeCardPorts | Target handle | Drop edge | Connection created | Both |
| TC-234 | Pipeline | PipelineNodeCardPorts | Source handle | Drag edge | Edge creation starts | Both |

### D.13 Pipeline Node Card Status

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-235 | Pipeline | PipelineNodeCardStatus | Status badge | Observe pending | Grey/neutral style | Both |
| TC-236 | Pipeline | PipelineNodeCardStatus | Status badge | Observe running | Blue/animated style | Both |
| TC-237 | Pipeline | PipelineNodeCardStatus | Status badge | Observe completed | Green/success style | Both |
| TC-238 | Pipeline | PipelineNodeCardStatus | Status badge | Observe failed | Red/error style | Both |

---

## E. Pipeline Inspectors & Dialogs

### E.1 Semantic Zoom Overlay

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-239 | Pipeline | SemanticZoomOverlay | Backdrop | Click | Overlay closes | Both |
| TC-240 | Pipeline | SemanticZoomOverlay | Failure section | Click expand | Error details shown | Both |
| TC-241 | Pipeline | SemanticZoomOverlay | Failure section | Click collapse | Error details hidden | Both |
| TC-242 | Pipeline | SemanticZoomOverlay | Fork button | Click | window.prompt for session ID | Both |

### E.2 Semantic Zoom Overlay Header

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-243 | Pipeline | SemanticZoomOverlayHeader | Close button | Click | Overlay closes | Both |
| TC-244 | Pipeline | SemanticZoomOverlayHeader | Close | Press Escape | Overlay closes | Both |

### E.3 Semantic Zoom Overlay Controls

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-245 | Pipeline | SemanticZoomOverlayControls | Fork button | Click | Fork initiated | Both |
| TC-246 | Pipeline | SemanticZoomOverlayControls | Restart button | Click | Node restarted | Both |
| TC-247 | Pipeline | SemanticZoomOverlayControls | Rewind button | Click | Node and downstream reset | Both |
| TC-248 | Pipeline | SemanticZoomOverlayControls | Abandon button | Click | Node abandoned | Both |
| TC-249 | Pipeline | SemanticZoomOverlayControls | Graft button | Click | Graft dialog opens | Both |
| TC-250 | Pipeline | SemanticZoomOverlayControls | Promote button | Click | Promotion dialog opens | Both |
| TC-251 | Pipeline | SemanticZoomOverlayControls | Reconnect 1 button | Click | Reconnect action 1 | Both |
| TC-252 | Pipeline | SemanticZoomOverlayControls | Reconnect 2 button | Click | Reconnect action 2 | Both |

### E.4 Graft Nodes Dialog

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-253 | Pipeline | GraftNodesDialog | Node type select | Open | Options listed | Both |
| TC-254 | Pipeline | GraftNodesDialog | Node type select | Select option | Type selected | Both |
| TC-255 | Pipeline | GraftNodesDialog | Custom ID input | Type text | Text appears | Both |
| TC-256 | Pipeline | GraftNodesDialog | Custom ID input | Leave empty | Validation warns | Both |
| TC-257 | Pipeline | GraftNodesDialog | Connect from select | Select node | Source selected | Both |
| TC-258 | Pipeline | GraftNodesDialog | Connect to select | Select node | Target selected | Both |
| TC-259 | Pipeline | GraftNodesDialog | Edge type select | Select type | Edge type set | Both |
| TC-260 | Pipeline | GraftNodesDialog | Type-specific fields | Fill all 15 bindings | All values captured | Both |
| TC-261 | Pipeline | GraftNodesDialog | Cancel button | Click | Dialog closes, no changes | Both |
| TC-262 | Pipeline | GraftNodesDialog | Submit button | Click with valid data | Graft applied | Both |
| TC-263 | Pipeline | GraftNodesDialog | Submit button | Click with invalid data | Validation errors shown | Both |
| TC-264 | Pipeline | GraftNodesDialog | Dialog | Press Escape | Dialog closes | Both |

### E.5 Join Pipeline Dialog

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-265 | Pipeline | JoinPipelineDialog | Source session ID input | Type ID | Text appears | Both |
| TC-266 | Pipeline | JoinPipelineDialog | Source session ID input | Paste ID | Pasted ID appears | Both |
| TC-267 | Pipeline | JoinPipelineDialog | Edge row source | Select source node | Source set | Both |
| TC-268 | Pipeline | JoinPipelineDialog | Edge row target | Select target node | Target set | Both |
| TC-269 | Pipeline | JoinPipelineDialog | Edge row type | Select edge type | Type set | Both |
| TC-270 | Pipeline | JoinPipelineDialog | Add Edge button | Click | New edge row added | Both |
| TC-271 | Pipeline | JoinPipelineDialog | Remove edge button | Click | Edge row removed | Both |
| TC-272 | Pipeline | JoinPipelineDialog | Cancel button | Click | Dialog closes, no join | Both |
| TC-273 | Pipeline | JoinPipelineDialog | Submit button | Click with valid edges | Pipelines joined | Both |
| TC-274 | Pipeline | JoinPipelineDialog | Submit button | Click with no edges | Validation error | Both |

### E.6 Promotion Dialog - Preview

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-275 | Pipeline | PromotionDialogPreview | Search input | Type query | Results filter | Both |
| TC-276 | Pipeline | PromotionDialogPreview | Append button | Click on result | Content appended to existing | Both |
| TC-277 | Pipeline | PromotionDialogPreview | Update button | Click on result | Existing updated | Both |
| TC-278 | Pipeline | PromotionDialogPreview | Cancel button | Click | Dialog closes | Both |
| TC-279 | Pipeline | PromotionDialogPreview | Create new button | Click | Proceeds to editor | Both |

### E.7 Promotion Dialog - Editor

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-280 | Pipeline | PromotionDialogEditor | Title input | Type text | Title captured | Both |
| TC-281 | Pipeline | PromotionDialogEditor | Title input | Leave empty | Validation error | Both |
| TC-282 | Pipeline | PromotionDialogEditor | Folder input | Type path | Folder set | Both |
| TC-283 | Pipeline | PromotionDialogEditor | Tags input | Type tags | Tags parsed | Both |
| TC-284 | Pipeline | PromotionDialogEditor | WikiLinks textarea | Type with [[links]] | Links highlighted | Both |
| TC-285 | Pipeline | PromotionDialogEditor | WikiLinks textarea | Type without [[links]] | Validation warning | Both |
| TC-286 | Pipeline | PromotionDialogEditor | Back button | Click | Returns to preview | Both |
| TC-287 | Pipeline | PromotionDialogEditor | Cancel button | Click | Dialog closes | Both |
| TC-288 | Pipeline | PromotionDialogEditor | Retry button | Click after error | Retry attempted | Both |
| TC-289 | Pipeline | PromotionDialogEditor | Continue button | Click with valid data | Proceeds to confirm | Both |

### E.8 Promotion Dialog - Confirm

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-290 | Pipeline | PromotionDialogConfirm | Back button | Click | Returns to editor | Both |
| TC-291 | Pipeline | PromotionDialogConfirm | Cancel button | Click | Dialog closes | Both |
| TC-292 | Pipeline | PromotionDialogConfirm | Retry button | Click after error | Retry promotion | Both |
| TC-293 | Pipeline | PromotionDialogConfirm | Edit button | Click | Returns to editor | Both |
| TC-294 | Pipeline | PromotionDialogConfirm | Submit button | Click | Memory created, dialog closes | Both |

### E.9 Edge Mapping Editor

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-295 | Pipeline | EdgeMappingEditor | Param input | Type param name | Value captured | Both |
| TC-296 | Pipeline | EdgeMappingEditor | Path input | Type JSON path | Value captured | Both |
| TC-297 | Pipeline | EdgeMappingEditor | Remove button | Click | Mapping row removed | Both |
| TC-298 | Pipeline | EdgeMappingEditor | Add mapping button | Click | New mapping row added | Both |

### E.10 Design Node Config Modal

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-299 | Pipeline | DesignNodeConfigModal | Close button | Click | Modal closes | Both |
| TC-300 | Pipeline | DesignNodeConfigModal | Panel | Observe | DesignNodeConfigPanel renders | Both |

### E.11 Design Node Config Panel

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-301 | Pipeline | DesignNodeConfigPanel | Label input | Type text | Label updates | Both |
| TC-302 | Pipeline | DesignNodeConfigPanel | Role select | Select role | Role applied | Both |
| TC-303 | Pipeline | DesignNodeConfigPanel | Color select | Select color | Color applied | Both |
| TC-304 | Pipeline | DesignNodeConfigPanel | Perspective select | Select perspective | Perspective applied | Both |
| TC-305 | Pipeline | DesignNodeConfigPanel | CWD input | Type path | Path set | Both |
| TC-306 | Pipeline | DesignNodeConfigPanel | Conditional edge inputs | Fill ~14 controls | All values captured | Both |
| TC-307 | Pipeline | DesignNodeConfigPanel | Form | Tab through | Logical focus order | Both |

### E.12 Add Node Form

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-308 | Pipeline | AddNodeForm | Type selector | Select prompt | Prompt fields shown | Both |
| TC-309 | Pipeline | AddNodeForm | Type selector | Select sequentialThinking | Thinking fields shown | Both |
| TC-310 | Pipeline | AddNodeForm | Type selector | Select workflow | Workflow fields shown | Both |
| TC-311 | Pipeline | AddNodeForm | Type selector | Select tool | Tool fields shown | Both |
| TC-312 | Pipeline | AddNodeFormTypeSelector | Submit button | Click | Proceeds to fields | Both |

### E.13 Add Node Form Fields

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-313 | Pipeline | AddNodeFormFields | Workflow ID input | Type ID | Value captured | Both |
| TC-314 | Pipeline | AddNodeFormFields | Thinking topic input | Type topic | Value captured | Both |
| TC-315 | Pipeline | AddNodeFormFields | Mode radio buttons | Select mode | Mode set | Both |
| TC-316 | Pipeline | AddNodeFormFields | Workflow prompt textarea | Type prompt | Value captured | Both |
| TC-317 | Pipeline | AddNodeFormFields | Prompt template textarea | Type template | Value captured | Both |
| TC-318 | Pipeline | AddNodeFormFields | Total thoughts number | Enter number | Value captured | Both |
| TC-319 | Pipeline | AddNodeFormFields | Total thoughts number | Enter invalid | Validation error | Both |
| TC-320 | Pipeline | AddNodeFormFields | Analysis type input | Type type | Value captured | Both |

### E.14 Tool Palette Fields

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-321 | Pipeline | ToolPaletteFields | Refresh button | Click | Tool list refreshes | Both |
| TC-322 | Pipeline | ToolPaletteFields | Search input | Type text | Tools filter | Both |
| TC-323 | Pipeline | ToolPaletteFields | Quick add button | Click | Tool added quickly | Both |
| TC-324 | Pipeline | ToolPaletteFields | Listbox item | Click | Tool selected | Both |
| TC-325 | Pipeline | ToolPaletteFields | Pin button | Click | Tool pinned | Both |
| TC-326 | Pipeline | ToolPaletteFields | Unpin button | Click | Tool unpinned | Both |
| TC-327 | Pipeline | ToolPaletteFields | Clear button | Click | Selection cleared | Both |
| TC-328 | Pipeline | ToolPaletteFields | Add Tool button | Click | Tool added to node | Both |
| TC-329 | Pipeline | ToolPaletteFields | Mode badges | Observe 16 events | Correct mode indicators | Both |
| TC-330 | Pipeline | ToolPaletteFields | All 16 controls | Exercise each | All function correctly | Both |

---

## F. Thinking/Cognitive Trace

### F.1 Thinking Trace View

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-331 | Thinking | ThinkingTraceView | Trace list | Observe | Items render in order | Both |
| TC-332 | Thinking | ThinkingTraceView | First 3 items | Observe | Details open by default | Both |
| TC-333 | Thinking | ThinkingTraceView | Item 4+ | Observe | Details collapsed by default | Both |
| TC-334 | Thinking | ThinkingTraceView | Markdown body | Observe | Markdown renders correctly | Both |

### F.2 Thinking Thought Item

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-335 | Thinking | ThinkingThoughtItem | Article | Observe | Read-only, well-formatted | Both |
| TC-336 | Thinking | ThinkingThoughtItem | WikiLinks | Observe | Links visually distinct | Both |

### F.3 Thinking Session Card

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-337 | Thinking | ThinkingSessionCard | Toggle button | Click | Session expands/collapses | Both |
| TC-338 | Thinking | ThinkingSessionCard | Card | Observe | Shows session ID, thought count | Both |

---

## G. Gate

### G.1 Gate View

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-339 | Gate | GateView | Container | Observe | Tasks displayed in table | Both |
| TC-340 | Gate | GateView | Container | Observe empty | "No tasks" message | Both |
| TC-341 | Gate | GateView | Container | Observe loading | Loading indicator | Both |

### G.2 Gate View Toolbar

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-342 | Gate | GateViewToolbar | Root select | Select root | Tasks filter to root | Both |
| TC-343 | Gate | GateViewToolbar | Browse button | Click | Folder picker opens | Electron |
| TC-344 | Gate | GateViewToolbar | Browse button | Click | Graceful failure | Cowork iframe ⚠️ |
| TC-345 | Gate | GateViewToolbar | Remove button | Click | Root removed | Both |
| TC-346 | Gate | GateViewToolbar | Refresh button | Click | Tasks reloaded | Both |

### G.3 Gate View Filters

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-347 | Gate | GateViewFilters | Search input | Type text | Tasks filter by text | Both |
| TC-348 | Gate | GateViewFilters | Search input | Clear | All tasks shown | Both |
| TC-349 | Gate | GateViewFilters | Process select | Select process | Tasks filter by process | Both |
| TC-350 | Gate | GateViewFilters | State select | Select state | Tasks filter by state | Both |
| TC-351 | Gate | GateViewFilters | Location select | Select location | Tasks filter by location | Both |
| TC-352 | Gate | GateViewFilters | Issues-only checkbox | Check | Only issues shown | Both |
| TC-353 | Gate | GateViewFilters | Issues-only checkbox | Uncheck | All tasks shown | Both |

### G.4 Gate Task Table

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-354 | Gate | GateTaskTable | Column header 1 | Click | Sort by column 1 | Both |
| TC-355 | Gate | GateTaskTable | Column header 1 | Click again | Sort direction reverses | Both |
| TC-356 | Gate | GateTaskTable | Column header 2 | Click | Sort by column 2 | Both |
| TC-357 | Gate | GateTaskTable | Column header 3 | Click | Sort by column 3 | Both |
| TC-358 | Gate | GateTaskTable | Column header 4 | Click | Sort by column 4 | Both |
| TC-359 | Gate | GateTaskTable | Task row | Click | Task selected, detail shown | Both |
| TC-360 | Gate | GateTaskTable | Task row | Arrow down when focused | Next row selected | Both |
| TC-361 | Gate | GateTaskTable | Task row | Arrow up when focused | Previous row selected | Both |

### G.5 Gate Task Detail

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-362 | Gate | GateTaskDetail | Clear/Close button | Click | Detail panel closes | Both |
| TC-363 | Gate | GateTaskDetail | Meta panel | Observe | Metadata displays | Both |
| TC-364 | Gate | GateTaskDetail | Stages panel | Observe | Stages display | Both |
| TC-365 | Gate | GateTaskDetail | History panel | Observe | History displays | Both |
| TC-366 | Gate | GateTaskDetail | Progress panel | Observe | Progress displays | Both |
| TC-367 | Gate | GateTaskDetail | Panels | Tab through | Focus moves logically | Both |

---

## H. Knowledge

### H.1 Knowledge Explorer

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-368 | Knowledge | KnowledgeExplorer | Search input | Type query | Text appears | Both |
| TC-369 | Knowledge | KnowledgeExplorer | Search input | Paste query | Pasted text appears | Both |
| TC-370 | Knowledge | KnowledgeExplorer | Search button | Click with query | Search executes | Both |
| TC-371 | Knowledge | KnowledgeExplorer | Search button | Press Enter in input | Search executes | Both |
| TC-372 | Knowledge | KnowledgeExplorer | Clear button | Click | Search cleared, all shown | Both |
| TC-373 | Knowledge | KnowledgeExplorer | Results | Observe after search | Relevant results displayed | Both |
| TC-374 | Knowledge | KnowledgeExplorer | No results | Search gibberish | "No results" message | Both |

### H.2 Knowledge Sidebar

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-375 | Knowledge | KnowledgeSidebar | Refresh button | Click | File tree refreshes | Both |
| TC-376 | Knowledge | KnowledgeSidebar | File open button | Click | File opens in viewer | Both |

### H.3 Knowledge Sidebar Node

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-377 | Knowledge | KnowledgeSidebarNode | Folder toggle | Click collapsed | Folder expands | Both |
| TC-378 | Knowledge | KnowledgeSidebarNode | Folder toggle | Click expanded | Folder collapses | Both |
| TC-379 | Knowledge | KnowledgeSidebarNode | File button | Click | File opens | Both |
| TC-380 | Knowledge | KnowledgeSidebarNode | Nested folders | Expand multiple levels | All levels visible | Both |

### H.4 Knowledge Note Renderer

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-381 | Knowledge | KnowledgeNoteRenderer | WikiLink | Click | Navigates to linked note | Both |
| TC-382 | Knowledge | KnowledgeNoteRenderer | WikiLink | Hover | Shows preview or tooltip | Both |
| TC-383 | Knowledge | KnowledgeNoteRenderer | Markdown | Observe | Renders headings, lists, code | Both |
| TC-384 | Knowledge | KnowledgeNoteRenderer | Broken link | Click | Graceful error handling | Both |

### H.5 Knowledge Graph Panel

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-385 | Knowledge | KnowledgeGraphPanel | Show community graph | Click | Graph visualization opens | Both |
| TC-386 | Knowledge | KnowledgeGraphPanel | Concept filter input | Type concept | Graph filters | Both |
| TC-387 | Knowledge | KnowledgeGraphPanel | Concept button | Click | Concept selected/highlighted | Both |
| TC-388 | Knowledge | KnowledgeGraphPanel | Reset button | Click | Graph resets to initial state | Both |
| TC-389 | Knowledge | KnowledgeGraphPanel | Reload button | Click | Graph data refreshes | Both |
| TC-390 | Knowledge | KnowledgeGraphPanel | Graph nodes | Click node | Node details shown | Both |
| TC-391 | Knowledge | KnowledgeGraphPanel | Graph | Pan/zoom | Canvas responds | Both |

---

## I. Workflow Authoring

### I.1 Workflow Authoring Page

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-392 | Workflows | WorkflowAuthoringPage | Container | Observe | Browser, Tree, Inspector, DraftBar render | Both |

### I.2 Workflow Browser

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-393 | Workflows | WorkflowBrowser | New workflow button | Click | New workflow created | Both |
| TC-394 | Workflows | WorkflowBrowser | Import button | Click | File input opens | Both |
| TC-395 | Workflows | WorkflowBrowser | Import file input | Select JSON file | Workflow imported | Both |
| TC-396 | Workflows | WorkflowBrowser | Import file input | Select invalid file | Validation error | Both |

### I.3 Workflow Browser List

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-397 | Workflows | WorkflowBrowserList | Workflow select button | Click | Workflow selected, details shown | Both |
| TC-398 | Workflows | WorkflowBrowserList | Workflow list | Scroll | All workflows accessible | Both |

### I.4 Workflow Browser Filters

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-399 | Workflows | WorkflowBrowserFilters | Search input | Type text | Workflows filter | Both |
| TC-400 | Workflows | WorkflowBrowserFilters | Category chip | Click | Filter by category | Both |
| TC-401 | Workflows | WorkflowBrowserFilters | Retry button | Click | Refresh after error | Both |
| TC-402 | Workflows | WorkflowBrowserFilters | Conflict resolution buttons | Click | Conflict resolved | Both |

### I.5 Workflow Draft Bar

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-403 | Workflows | WorkflowDraftBar | Cancel button | Click | Changes cancelled | Both |
| TC-404 | Workflows | WorkflowDraftBar | Discard button | Click | Confirm dialog shown | Both |
| TC-405 | Workflows | WorkflowDraftBar | Discard confirm | Confirm | Changes discarded 🔴 | Both |
| TC-406 | Workflows | WorkflowDraftBar | Save button | Click | Workflow saved | Both |
| TC-407 | Workflows | WorkflowDraftBar | Save button | Observe when clean | Button disabled | Both |

### I.6 Workflow Metadata Form

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-408 | Workflows | WorkflowMetadataForm | Name input | Type name | Value captured | Both |
| TC-409 | Workflows | WorkflowMetadataForm | Name input | Leave empty | Validation error | Both |
| TC-410 | Workflows | WorkflowMetadataForm | Emoji input | Enter emoji | Emoji captured | Both |
| TC-411 | Workflows | WorkflowMetadataForm | Description input | Type text | Value captured | Both |
| TC-412 | Workflows | WorkflowMetadataForm | Triggers input | Type triggers | Value captured | Both |
| TC-413 | Workflows | WorkflowMetadataForm | Category select | Select category | Category set | Both |
| TC-414 | Workflows | WorkflowMetadataForm | Toggle 1 | Toggle on/off | State changes | Both |
| TC-415 | Workflows | WorkflowMetadataForm | Toggle 2 | Toggle on/off | State changes | Both |

### I.7 Workflow Step Timeline

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-416 | Workflows | WorkflowStepTimeline | Toggle all button | Click | All steps expand/collapse | Both |
| TC-417 | Workflows | WorkflowStepTimeline | Export button | Click | Export options shown | Both |
| TC-418 | Workflows | WorkflowStepTimeline | Export sub-buttons | Click each | Export in format | Both |
| TC-419 | Workflows | WorkflowStepTimeline | Save button | Click | Steps saved | Both |
| TC-420 | Workflows | WorkflowStepTimeline | Add step button | Click | New step added | Both |

### I.8 Workflow Step Card

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-421 | Workflows | WorkflowStepCard | Expand button | Click | Step details expand | Both |
| TC-422 | Workflows | WorkflowStepCard | Collapse button | Click | Step details collapse | Both |
| TC-423 | Workflows | WorkflowStepCard | Move up button | Click | Step moves up | Both |
| TC-424 | Workflows | WorkflowStepCard | Move up button | Observe on first step | Button disabled | Both |
| TC-425 | Workflows | WorkflowStepCard | Move down button | Click | Step moves down | Both |
| TC-426 | Workflows | WorkflowStepCard | Move down button | Observe on last step | Button disabled | Both |
| TC-427 | Workflows | WorkflowStepCard | Remove button | Click | Confirm dialog shown | Both |
| TC-428 | Workflows | WorkflowStepCard | Confirm remove Cancel | Click | Step preserved | Both |
| TC-429 | Workflows | WorkflowStepCard | Confirm remove Remove | Click | Step removed 🔴 | Both |
| TC-430 | Workflows | WorkflowStepCard | Card | Drag | Card repositions (if drag enabled) | Both |

### I.9 Workflow Step Inspector

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-431 | Workflows | WorkflowStepInspector | Name input | Type name | Value captured | Both |
| TC-432 | Workflows | WorkflowStepInspector | Description textarea | Type text | Value captured | Both |
| TC-433 | Workflows | WorkflowStepInspector | Thinking toggle | Toggle | State changes | Both |
| TC-434 | Workflows | WorkflowStepInspector | Effort segment | Click low | Low effort set | Both |
| TC-435 | Workflows | WorkflowStepInspector | Effort segment | Click medium | Medium effort set | Both |
| TC-436 | Workflows | WorkflowStepInspector | Effort segment | Click high | High effort set | Both |
| TC-437 | Workflows | WorkflowStepInspector | Form | Tab through | Logical focus order | Both |

---

## J. Agents & Templates

### J.1 Agent Selector

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-438 | Various | AgentSelector | Dropdown | Click | Agent list opens | Both |
| TC-439 | Various | AgentSelector | Agent option | Click | Agent selected | Both |
| TC-440 | Various | AgentSelector | Dropdown | Press Escape | Closes without selecting | Both |
| TC-441 | Various | AgentSelector | Dropdown | Arrow keys | Navigate options | Both |

### J.2 Agent Probe Capabilities

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-442 | Agents | AgentProbeCapabilities | Capability select | Select capability | Capability selected | Both |
| TC-443 | Agents | AgentProbeCapabilities | Multiple capabilities | Select several | All selections shown | Both |

### J.3 Agent Probe Status

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-444 | Agents | AgentProbeStatus | Refresh button | Click | Status refreshes | Both |
| TC-445 | Agents | AgentProbeStatus | Status | Observe connected | Green/connected style | Both |
| TC-446 | Agents | AgentProbeStatus | Status | Observe disconnected | Red/disconnected style | Both |

### J.4 Template Gallery Card

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-447 | Templates | TemplateGalleryCard | Select button | Click | Template selected | Both |
| TC-448 | Templates | TemplateGalleryCard | Card | Observe | Title, description visible | Both |

### J.5 Template Gallery Selection

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-449 | Templates | TemplateGallerySelection | Create button | Click | Template applied | Both |
| TC-450 | Templates | TemplateGallerySelection | Selection | Observe | Selected template highlighted | Both |

### J.6 Template Gallery Intent Form

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-451 | Templates | TemplateGalleryIntentForm | Intent input | Type intent | Value captured | Both |
| TC-452 | Templates | TemplateGalleryIntentForm | Submit button | Click with text | Intent submitted | Both |
| TC-453 | Templates | TemplateGalleryIntentForm | Submit button | Click when empty | Validation error | Both |

---

## K. Settings

### K.1 Settings View

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-454 | Settings | SettingsView | Section nav button 1 | Click | Section 1 scrolls into view | Both |
| TC-455 | Settings | SettingsView | Section nav button 2 | Click | Section 2 scrolls into view | Both |
| TC-456 | Settings | SettingsView | Section nav button 3 | Click | Section 3 scrolls into view | Both |
| TC-457 | Settings | SettingsView | All sections | Scroll | All sections accessible | Both |

### K.2 Settings Auth Identity

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-458 | Settings | SettingsAuthIdentity | Provider select | Select provider | Provider set | Both |
| TC-459 | Settings | SettingsAuthIdentity | Name input | Type name | Value captured | Both |
| TC-460 | Settings | SettingsAuthIdentity | Email input | Type email | Value captured | Both |
| TC-461 | Settings | SettingsAuthIdentity | Email input | Type invalid email | Validation error | Both |
| TC-462 | Settings | SettingsAuthIdentity | Tenant input | Type tenant | Value captured | Both |
| TC-463 | Settings | SettingsAuthIdentity | Token input | Type token | Value masked/captured | Both |
| TC-464 | Settings | SettingsAuthIdentity | Token input | Paste token | Pasted value captured | Both |
| TC-465 | Settings | SettingsAuthIdentity | Token status select | Select status | Status set | Both |
| TC-466 | Settings | SettingsAuthIdentity | Scope textarea | Type scopes | Value captured | Both |
| TC-467 | Settings | SettingsAuthIdentity | Switch 1 | Toggle | State changes | Both |
| TC-468 | Settings | SettingsAuthIdentity | Switch 2 | Toggle | State changes | Both |
| TC-469 | Settings | SettingsAuthIdentity | Switch 3 | Toggle | State changes | Both |
| TC-470 | Settings | SettingsAuthIdentity | Sign out button | Click | Confirm dialog 🔴 | Both |
| TC-471 | Settings | SettingsAuthIdentity | Sign out confirm | Confirm | Signed out, session cleared 🔴 | Both |
| TC-472 | Settings | SettingsAuthIdentity | Mark reconnected | Click | Connection marked | Both |

### K.3 Settings Connection Agents

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-473 | Settings | SettingsConnectionAgents | +Add button | Click | New agent row added | Both |
| TC-474 | Settings | SettingsConnectionAgents | Name input | Type name | Value captured | Both |
| TC-475 | Settings | SettingsConnectionAgents | Command input | Type command | Value captured | Both |
| TC-476 | Settings | SettingsConnectionAgents | Save button | Click | Agent saved | Both |
| TC-477 | Settings | SettingsConnectionAgents | Cancel button | Click | Edit cancelled | Both |
| TC-478 | Settings | SettingsConnectionAgents | Edit button | Click | Row enters edit mode | Both |
| TC-479 | Settings | SettingsConnectionAgents | Delete button | Click | Agent deleted 🔴 | Both |

### K.4 Settings Connection Daemon

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-480 | Settings | SettingsConnectionDaemon | Refresh button | Click | Daemon status refreshes | Both |
| TC-481 | Settings | SettingsConnectionDaemon | Copy address button | Click | Address copied to clipboard 📋 | Both |
| TC-482 | Settings | SettingsConnectionDaemon | Status | Observe running | Running indicator | Both |
| TC-483 | Settings | SettingsConnectionDaemon | Status | Observe stopped | Stopped indicator | Both |

### K.5 Settings Connection Keys

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-484 | Settings | SettingsConnectionKeys | Env row remove | Click | Variable removed 🔴 | Both |
| TC-485 | Settings | SettingsConnectionKeys | Add Variable button | Click | New env row added | Both |
| TC-486 | Settings | SettingsConnectionKeys | Env name input | Type name | Value captured | Both |
| TC-487 | Settings | SettingsConnectionKeys | Env value input | Type value | Value captured | Both |
| TC-488 | Settings | SettingsConnectionKeys | MCP server remove | Click | Server removed 🔴 | Both |
| TC-489 | Settings | SettingsConnectionKeys | MCP type select | Select type | Type set | Both |
| TC-490 | Settings | SettingsConnectionKeys | MCP name input | Type name | Value captured | Both |
| TC-491 | Settings | SettingsConnectionKeys | MCP command input | Type command | Value captured | Both |
| TC-492 | Settings | SettingsConnectionKeys | MCP URL input | Type URL | Value captured | Both |
| TC-493 | Settings | SettingsConnectionKeys | MCP args input | Type args | Value captured | Both |
| TC-494 | Settings | SettingsConnectionKeys | Add MCP Server button | Click | New server row added | Both |

### K.6 Settings Maenifold

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-495 | Settings | SettingsMaenifold | Save button | Click | Settings saved | Both |
| TC-496 | Settings | SettingsMaenifold | Reset button | Click | Settings reset to defaults | Both |
| TC-497 | Settings | SettingsMaenifold | 20+ controls | Exercise each | All values captured correctly | Both |
| TC-498 | Settings | SettingsMaenifold | Number input | Enter invalid | Validation error | Both |
| TC-499 | Settings | SettingsMaenifold | Checkbox | Check/uncheck | State toggles | Both |
| TC-500 | Settings | SettingsMaenifold | Select | Select option | Option set | Both |

### K.7 Settings MCP Servers List

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-501 | Settings | SettingsMcpServersList | +Add button | Click | Server detail form opens | Both |
| TC-502 | Settings | SettingsMcpServersList | Edit button | Click | Server detail form opens with data | Both |
| TC-503 | Settings | SettingsMcpServersList | Delete button | Click | Server deleted 🔴 | Both |

### K.8 Settings MCP Servers Detail

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-504 | Settings | SettingsMcpServersDetail | Name input | Type name | Value captured | Both |
| TC-505 | Settings | SettingsMcpServersDetail | Type input | Type type | Value captured | Both |
| TC-506 | Settings | SettingsMcpServersDetail | Command input | Type command | Value captured | Both |
| TC-507 | Settings | SettingsMcpServersDetail | Args input | Type args | Value captured | Both |
| TC-508 | Settings | SettingsMcpServersDetail | URL input | Type URL | Value captured | Both |
| TC-509 | Settings | SettingsMcpServersDetail | Save button | Click with valid | Server saved | Both |
| TC-510 | Settings | SettingsMcpServersDetail | Save button | Click with invalid | Validation errors | Both |
| TC-511 | Settings | SettingsMcpServersDetail | Cancel button | Click | Form closes, no save | Both |

### K.9 Settings Preferences

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-512 | Settings | SettingsPreferences | Telemetry checkbox | Check | Telemetry enabled | Both |
| TC-513 | Settings | SettingsPreferences | Telemetry checkbox | Uncheck | Telemetry disabled | Both |
| TC-514 | Settings | SettingsPreferences | Clear last cwd button | Click | CWD cleared | Both |

---

## L. Diagnostics & Traffic

### L.1 Traffic Monitor

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-515 | Traffic | TrafficMonitor | Resize handle | Drag | Panel resizes | Both |
| TC-516 | Traffic | TrafficMonitor | Pause button | Click | Traffic capture paused | Both |
| TC-517 | Traffic | TrafficMonitor | Resume button | Click | Traffic capture resumes | Both |
| TC-518 | Traffic | TrafficMonitor | Clear button | Click | All entries cleared | Both |
| TC-519 | Traffic | TrafficMonitor | Filter button 1 | Click | Filter applied | Both |
| TC-520 | Traffic | TrafficMonitor | Filter button 2 | Click | Filter applied | Both |
| TC-521 | Traffic | TrafficMonitor | Direction button 1 | Click | Direction filter applied | Both |
| TC-522 | Traffic | TrafficMonitor | Direction button 2 | Click | Direction filter applied | Both |
| TC-523 | Traffic | TrafficMonitor | Search input | Type text | Entries filter | Both |
| TC-524 | Traffic | TrafficMonitor | Clear search button | Click | Search cleared | Both |
| TC-525 | Traffic | TrafficMonitor | Close button | Click | Traffic monitor closes | Both |

### L.2 Traffic Entry

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-526 | Traffic | TrafficEntry | Header | Click | Entry expands | Both |
| TC-527 | Traffic | TrafficEntry | Header | Click when expanded | Entry collapses | Both |
| TC-528 | Traffic | TrafficEntry | Copy button | Click | Content copied to clipboard 📋 | Both |
| TC-529 | Traffic | TrafficEntry | Entry | Observe | Timestamp, method, direction visible | Both |

---

## M. Cross-Cutting & Edge Cases

### M.1 Keyboard Navigation

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-530 | All | All | Focusable elements | Tab through page | Focus order is logical | Both |
| TC-531 | All | All | Focusable elements | Shift+Tab | Reverse focus order works | Both |
| TC-532 | All | All | Focus indicator | Tab to element | Visible focus ring/outline | Both |
| TC-533 | All | All | Skip links | Tab from top | Skip to main content available | Both |

### M.2 Loading States

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-534 | All | Various | Loading state | Observe during fetch | Loading indicator visible | Both |
| TC-535 | All | Various | Loading state | Observe completion | Loading indicator disappears | Both |
| TC-536 | All | Various | Buttons during load | Observe | Buttons disabled during operation | Both |

### M.3 Error States

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-537 | All | Various | Network error | Disconnect network | Error message with retry | Both |
| TC-538 | All | Various | Server error | Trigger 500 error | Error message displayed | Both |
| TC-539 | All | Various | Validation error | Submit invalid data | Field-level errors shown | Both |
| TC-540 | All | Various | Retry button | Click after error | Operation retried | Both |

### M.4 Empty States

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-541 | Pipeline | PipelineView | No pipelines | Observe | "Create your first pipeline" prompt | Both |
| TC-542 | Knowledge | KnowledgeExplorer | No notes | Observe | "No knowledge found" prompt | Both |
| TC-543 | Workflows | WorkflowBrowser | No workflows | Observe | "Create a workflow" prompt | Both |
| TC-544 | Chat | ChatView | No messages | Observe | Welcome/starter prompt | Both |

### M.5 Max Length & Boundary Conditions

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-545 | Various | Inputs | Text input | Enter 10,000+ characters | Handles gracefully (truncate or scroll) | Both |
| TC-546 | Various | Inputs | Number input | Enter very large number | Validation or clamping | Both |
| TC-547 | Various | Inputs | Number input | Enter negative number | Validation if not allowed | Both |
| TC-548 | Various | Lists | Long list | 1000+ items | Virtualizes or paginates | Both |

### M.6 Visual Design Tokens

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-549 | All | All | Colors | Observe | Consistent color palette | Both |
| TC-550 | All | All | Typography | Observe | Consistent font sizes/weights | Both |
| TC-551 | All | All | Spacing | Observe | Consistent padding/margins | Both |
| TC-552 | All | All | Shadows | Observe | Consistent elevation/shadows | Both |
| TC-553 | All | All | Border radius | Observe | Consistent corner rounding | Both |

### M.7 Z-Index Layering

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-554 | All | Dialogs | Modal | Open modal | Modal above all content | Both |
| TC-555 | All | Toasts | Toast | Trigger toast | Toast above modal | Both |
| TC-556 | All | Dropdowns | Select | Open dropdown | Dropdown above siblings | Both |
| TC-557 | All | Context menus | Right-click menu | Open menu | Menu above all content | Both |
| TC-558 | All | Command palette | Palette | Open Cmd+K | Palette above all content | Both |

### M.8 Overflow & Clipping

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-559 | All | Text | Long text | Enter very long text | Text truncates with ellipsis or wraps | Both |
| TC-560 | All | Lists | Overflow | Many items | Scroll appears, no clipping | Both |
| TC-561 | All | Cards | Content overflow | Long content | Card expands or scrolls | Both |

### M.9 Responsive Behavior

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-562 | All | Layout | Window | Resize to 1920px | Full desktop layout | Electron |
| TC-563 | All | Layout | Window | Resize to 1024px | Adjusted layout | Electron |
| TC-564 | All | Layout | Window | Resize to 768px | Mobile/tablet adaptations | Both |
| TC-565 | All | Sidebar | Sidebar | At narrow width | Collapses or overlays | Both |

### M.10 Clipboard Operations

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-566 | Various | Copy buttons | Any copy button | Click | Content copied to clipboard 📋 | Both |
| TC-567 | Various | Inputs | Any input | Cmd+V / Ctrl+V | Content pastes | Both |
| TC-568 | Various | Inputs | Any input | Cmd+C / Ctrl+C | Selected content copies | Both |
| TC-569 | Various | Inputs | Any input | Cmd+X / Ctrl+X | Selected content cuts | Both |

### M.11 Undo/Redo

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-570 | Various | Inputs | Text input | Cmd+Z / Ctrl+Z | Last input undone | Both |
| TC-571 | Various | Inputs | Text input | Cmd+Shift+Z / Ctrl+Y | Redo works | Both |

---

## O. Onboarding Wizard

### O.1 Wizard Gate & Lifecycle

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-579 | App | App.vue | Onboarding gate | First launch (no localStorage) | Wizard displays full-screen, blocking app | Both |
| TC-580 | App | App.vue | Onboarding gate | After completing onboarding | Main app displays, wizard hidden | Both |
| TC-581 | App | App.vue | Onboarding gate | Clear localStorage, reload | Wizard displays again | Both |

### O.2 Wizard Shell & Navigation

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-582 | Wizard | OnboardingWizard | Left sidebar | Observe | Shows 5 steps: Welcome, Daemon, Agent, Model, Finish | Both |
| TC-583 | Wizard | OnboardingWizard | Step indicators | Observe current step | Current step highlighted with brand color ring | Both |
| TC-584 | Wizard | OnboardingWizard | Step indicators | Observe completed steps | Completed steps show green checkmark | Both |
| TC-585 | Wizard | OnboardingWizard | Step indicators | Click completed step | Navigates back to that step | Both |
| TC-586 | Wizard | OnboardingWizard | Step indicators | Click future step | Does nothing (disabled) | Both |
| TC-587 | Wizard | OnboardingWizard | Step transition | Advance to next step | Smooth slide animation (opacity + translateY) | Both |

### O.3 Welcome Step

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-588 | Wizard | OnboardingStepWelcome | Heading | Observe | Shows welcome text | Both |
| TC-589 | Wizard | OnboardingStepWelcome | Continue button | Click | Advances to Daemon step | Both |

### O.4 Daemon Step

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-590 | Wizard | OnboardingStepDaemon | Auto-probe on mount | Observe | Probes daemon via `GET /api/config` (not `/health`) | Both |
| TC-591 | Wizard | OnboardingStepDaemon | Connected state | Observe when daemon running | Green success box with agent/MCP server counts | Both |
| TC-592 | Wizard | OnboardingStepDaemon | Disconnected state | Observe when daemon down | Red dot, "Disconnected", correct start command | Both |
| TC-593 | Wizard | OnboardingStepDaemon | Start command | Observe text | Shows `maenifold --http` (NOT `maenifold-cli serve`) | Both |
| TC-594 | Wizard | OnboardingStepDaemon | Copy button | Click | Copies correct command to clipboard 📋 | Both |
| TC-595 | Wizard | OnboardingStepDaemon | Copy button | After click | Shows "✓ Copied!" for 2 seconds | Both |
| TC-596 | Wizard | OnboardingStepDaemon | Check connection | Click | Re-probes daemon | Both |
| TC-597 | Wizard | OnboardingStepDaemon | Continue button | Observe when disconnected | Button disabled | Both |
| TC-598 | Wizard | OnboardingStepDaemon | Continue button | Click when connected | Advances to Agent step | Both |
| TC-599 | Wizard | OnboardingStepDaemon | Auto-poll | Wait 5 seconds while disconnected | Re-probes automatically | Both |
| TC-600 | Wizard | OnboardingStepDaemon | Back button | Click | Returns to Welcome step | Both |

### O.5 Agent Step

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-601 | Wizard | OnboardingStepAgent | Agent name input | Observe default | Pre-filled with "Copilot" or similar | Both |
| TC-602 | Wizard | OnboardingStepAgent | Agent command input | Observe | Text input for full shell command, placeholder shows real ACP command | Both |
| TC-603 | Wizard | OnboardingStepAgent | Agent command input | Observe placeholder | Shows `npx @github/copilot-language-server@latest --acp` or similar real command | Both |
| TC-604 | Wizard | OnboardingStepAgent | Add agent | Click with valid name+command | Agent added via `POST /api/config/agents`, advances | Both |
| TC-605 | Wizard | OnboardingStepAgent | Duplicate name | Enter existing agent name | Error message shown, button disabled | Both |
| TC-606 | Wizard | OnboardingStepAgent | Empty name | Clear name field | Button disabled | Both |
| TC-607 | Wizard | OnboardingStepAgent | Skip/existing | Observe when agents already configured | Shows existing agents, allows continuing | Both |
| TC-608 | Wizard | OnboardingStepAgent | Back button | Click | Returns to Daemon step | Both |

### O.6 Model Step

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-609 | Wizard | OnboardingStepModel | Model list | Observe on mount | Loads models via `EmbeddingModels` MCP tool | Both |
| TC-610 | Wizard | OnboardingStepModel | Model cards | Observe | Shows model name, dimensions, max tokens, size | Both |
| TC-611 | Wizard | OnboardingStepModel | Active model badge | Observe | Currently active model shows "active" badge | Both |
| TC-612 | Wizard | OnboardingStepModel | Select model | Click model card | Radio indicator selects that model | Both |
| TC-613 | Wizard | OnboardingStepModel | Download button | Click | Downloads selected model, shows progress | Both |
| TC-614 | Wizard | OnboardingStepModel | Skip button | Click "Download later" | Advances to Finish step, model marked as skipped | Both |
| TC-615 | Wizard | OnboardingStepModel | Dimension warning | Observe | Warning about re-embedding if dimensions change | Both |
| TC-616 | Wizard | OnboardingStepModel | Load error | Observe when MCP unavailable | Error message with retry button | Both |
| TC-617 | Wizard | OnboardingStepModel | Back button | Click | Returns to Agent step | Both |

### O.7 Finish Step

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-618 | Wizard | OnboardingStepDone | Summary card | Observe | Shows daemon ✓, agent name, model status, config path | Both |
| TC-619 | Wizard | OnboardingStepDone | Model skipped | Observe when model was skipped | Warning indicator on model row | Both |
| TC-620 | Wizard | OnboardingStepDone | Config path | Observe | Shows real config path from store (NOT hardcoded) | Both |
| TC-621 | Wizard | OnboardingStepDone | Documentation link | Click | Opens GitHub repo in new tab | Both |
| TC-622 | Wizard | OnboardingStepDone | Start button | Click | Probes daemon, completes onboarding, shows main app | Both |
| TC-623 | Wizard | OnboardingStepDone | Start button | Click when daemon down | Warning toast, does not complete | Both |
| TC-624 | Wizard | OnboardingStepDone | Back button | Click | Returns to Model step | Both |
| TC-625 | Wizard | OnboardingStepDone | Glow animation | Observe | Subtle brand gradient glow behind content | Both |

### O.8 Design Compliance

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-626 | Wizard | All steps | Typography | Observe headings | 28px bold tracking-tight text-text-primary | Both |
| TC-627 | Wizard | All steps | Body text | Observe | text-sm text-text-secondary leading-relaxed | Both |
| TC-628 | Wizard | All steps | Borders | Observe | Tonal surface shifts only, no `1px solid` hard borders | Both |
| TC-629 | Wizard | All steps | Buttons | Observe primary | Gradient brand bg, dark text, 4px radius | Both |
| TC-630 | Wizard | All steps | Status dots | Observe | 6px dots (not pill badges) | Both |
| TC-631 | Wizard | All steps | Shadows | Observe | No drop shadows (ambient glow only) | Both |
| TC-632 | Wizard | All steps | Casing | Observe all text | Sentence case (never ALL CAPS) | Both |

---

## N. Context-Specific Tests (Cowork iframe)

### N.1 Iframe Limitations

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-572 | Various | folder-picker | Any trigger | Click | Graceful failure or fallback UI | Cowork iframe ⚠️ |
| TC-573 | Various | Native dialogs | File inputs | Click | Browser file picker works | Cowork iframe |
| TC-574 | Settings | SettingsConnectionDaemon | Copy address | Click | Clipboard API works | Cowork iframe |
| TC-575 | All | Links | External links | Click | Opens in new tab (not iframe) | Cowork iframe |

### N.2 Iframe Communication

| TC-ID | Screen | Component | Element | Action | Expected Result | Context |
|-------|--------|-----------|---------|--------|-----------------|---------|
| TC-576 | Shell | Host | postMessage | Send from host | App receives message | Cowork iframe |
| TC-577 | Shell | App | postMessage | Send to host | Host receives message | Cowork iframe |
| TC-578 | Shell | Resize | Host resize | Resize iframe | App layout adjusts | Cowork iframe |

---

## Appendix A: Test Execution Checklist

### Pre-Test Setup

- [ ] Electron app installed and running
- [ ] Cowork host available with iframe embedded
- [ ] Maenifold daemon running
- [ ] At least one agent configured
- [ ] Sample pipelines and workflows available
- [ ] Knowledge corpus populated with test files

### Test Browsers (for Cowork iframe)

- [ ] Chrome (latest)
- [ ] Edge (latest)
- [ ] Safari (latest, if applicable)

### Test Recording

For each test case, record:
- Pass / Fail / Blocked
- Actual result (if different from expected)
- Screenshots for failures
- Browser console errors
- Network request failures

---

## Appendix B: Severity Classification

| Severity | Definition |
|----------|------------|
| Critical | App crash, data loss, security vulnerability |
| High | Major feature broken, no workaround |
| Medium | Feature impaired, workaround exists |
| Low | Cosmetic issue, minor inconvenience |

---

## Appendix C: Test Case Count Summary

| Category | Count |
|----------|-------|
| A. Application Shell | 75 |
| B. Permissions & Auth Dialogs | 27 |
| C. Chat Surface | 56 |
| D. Pipeline View & Composer | 76 |
| E. Pipeline Inspectors & Dialogs | 38 |
| F. Thinking/Cognitive Trace | 8 |
| G. Gate | 29 |
| H. Knowledge | 24 |
| I. Workflow Authoring | 46 |
| J. Agents & Templates | 17 |
| K. Settings | 61 |
| L. Diagnostics & Traffic | 15 |
| M. Cross-Cutting & Edge Cases | 42 |
| N. Context-Specific (iframe) | 7 |
| O. Onboarding Wizard | 54 |
| **Total** | **575** |

---

*Document generated by QA Architect*  
*Last updated: 2026-04-19*
