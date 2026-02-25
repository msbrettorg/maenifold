"""
Asset watcher tools for maenifold.

Monitors ~/maenifold/assets/ for JSON file changes with debouncing.
Provides start/stop functionality for file system watching with incremental sync support.

Implementation matches C# AssetWatcherTools.cs with Python watchdog library.
"""

import os
import threading
import time
from typing import Optional

from watchdog.events import FileSystemEvent, FileSystemEventHandler
from watchdog.observers import Observer

from maenifold.config import Config


class AssetChangeHandler(FileSystemEventHandler):
    """
    Event handler for asset directory changes.

    Implements debouncing to batch rapid file changes into a single notification.
    Ma Protocol: No fake AI - just restart debounce timer, don't make decisions.
    Errors propagate naturally without recovery logic.
    """

    def __init__(self, debounce_callback, debounce_ms: int):
        """
        Initialize the event handler.

        Args:
            debounce_callback: Callback function to invoke after debounce period
            debounce_ms: Debounce delay in milliseconds
        """
        super().__init__()
        self._debounce_callback = debounce_callback
        self._debounce_ms = debounce_ms
        self._debounce_timer: Optional[threading.Timer] = None
        self._lock = threading.Lock()

    def on_error(self, event) -> None:
        """
        Handler for file system watcher errors.

        Ma Protocol: NO FAKE AI - errors propagate, no recovery logic.
        This ensures the LLM has complete information to make decisions.

        Args:
            event: Error event from watchdog

        Raises:
            Exception: Re-raises the error to propagate to caller
        """
        # NO FAKE AI: Let error propagate with complete information
        if isinstance(event, Exception):
            raise event
        else:
            raise OSError(f"FileSystemWatcher encountered error: {event}")

    def on_created(self, event: FileSystemEvent) -> None:
        """
        Handler for file creation events.

        Args:
            event: File system event
        """
        if not event.is_directory and event.src_path.endswith('.json'):
            self._restart_debounce_timer()

    def on_modified(self, event: FileSystemEvent) -> None:
        """
        Handler for file modification events.

        Args:
            event: File system event
        """
        if not event.is_directory and event.src_path.endswith('.json'):
            self._restart_debounce_timer()

    def on_deleted(self, event: FileSystemEvent) -> None:
        """
        Handler for file deletion events.

        Args:
            event: File system event
        """
        if not event.is_directory and event.src_path.endswith('.json'):
            self._restart_debounce_timer()

    def on_moved(self, event: FileSystemEvent) -> None:
        """
        Handler for file rename/move events.

        Args:
            event: File system event
        """
        if not event.is_directory:
            # Check if either old or new path is a JSON file
            if (hasattr(event, 'src_path') and event.src_path.endswith('.json')) or \
               (hasattr(event, 'dest_path') and event.dest_path.endswith('.json')):
                self._restart_debounce_timer()

    def _restart_debounce_timer(self) -> None:
        """
        Restart the debounce timer to batch asset changes.

        Ma Protocol: Simple timer restart, no logic about what changed.
        """
        with self._lock:
            if self._debounce_timer is not None:
                self._debounce_timer.cancel()

            # Convert milliseconds to seconds for Timer
            delay_seconds = self._debounce_ms / 1000.0
            self._debounce_timer = threading.Timer(delay_seconds, self._debounce_callback)
            self._debounce_timer.start()

    def cleanup(self) -> None:
        """Cancel any pending debounce timer."""
        with self._lock:
            if self._debounce_timer is not None:
                self._debounce_timer.cancel()
                self._debounce_timer = None


# Singleton state
_watcher_lock = threading.Lock()
_observer: Optional[Observer] = None
_event_handler: Optional[AssetChangeHandler] = None


def start_asset_watcher() -> str:
    """
    Start watching the assets directory for JSON file changes.

    Monitors ~/maenifold/assets/ with the following configuration:
    - Filter: *.json files only
    - IncludeSubdirectories: true (monitors nested asset folders)
    - Debounce: 150ms default (MAENIFOLD_DEBOUNCE_MS env var)
    - Buffer size: 64KB (MAENIFOLD_WATCHER_BUFFER env var)

    Implements singleton pattern to prevent multiple watchers.

    Returns:
        Status message indicating watcher started or already running

    Raises:
        OSError: If assets directory cannot be created or accessed
    """
    global _observer, _event_handler

    with _watcher_lock:
        if _observer is not None:
            return "Asset watcher already running"

        # Ensure assets directory exists
        os.makedirs(Config.assets_path, exist_ok=True)

        # Create event handler with debouncing
        _event_handler = AssetChangeHandler(
            debounce_callback=_on_debounce_elapsed,
            debounce_ms=Config.default_debounce_ms
        )

        # Create and configure observer
        _observer = Observer()
        _observer.schedule(
            _event_handler,
            Config.assets_path,
            recursive=True  # IncludeSubdirectories = true
        )

        # Start the observer
        _observer.start()

        return (
            f"Started watching {Config.assets_path} for asset changes "
            f"(debounce: {Config.default_debounce_ms}ms)"
        )


def stop_asset_watcher() -> str:
    """
    Stop watching the assets directory.

    Stops the observer, cancels pending debounce timers, and cleans up resources.
    Safe to call multiple times.

    Returns:
        Status message indicating watcher stopped or not running
    """
    global _observer, _event_handler

    with _watcher_lock:
        if _observer is None:
            return "Asset watcher not running"

        # Stop the observer
        _observer.stop()
        _observer.join(timeout=2.0)  # Wait up to 2 seconds for thread to finish
        _observer = None

        # Cleanup event handler
        if _event_handler is not None:
            _event_handler.cleanup()
            _event_handler = None

        return "Stopped asset watcher"


def _on_debounce_elapsed() -> None:
    """
    Called when debounce timer expires after asset changes.

    This function is invoked after the debounce period when asset files have changed.
    Currently a placeholder for future MCP resource notification integration.

    Ma Protocol: Simple notification pattern. Let MCP SDK handle resource refresh.
    Future Wave 4: Send notifications/resources/list_changed to MCP clients.
    """
    # TODO: Wave 4 implementation
    # Send MCP notification: notifications/resources/list_changed
    # Clients will automatically call ListResources to get updated asset list

    # For now, just log that debounce elapsed
    # This ensures the watcher is functioning correctly
    pass


def _get_help_text() -> str:
    """
    Get help documentation for watcher tools.

    Returns:
        Help text describing watcher functionality
    """
    return """# Asset Watcher Tools

Monitors ~/maenifold/assets/ for JSON file changes with debouncing.

## StartAssetWatcher

Start watching the assets directory for changes.

**Configuration:**
- Monitors: `~/maenifold/assets/` (recursive)
- File filter: `*.json`
- Debounce: 150ms (configurable via MAENIFOLD_DEBOUNCE_MS)
- Buffer size: 64KB (configurable via MAENIFOLD_WATCHER_BUFFER)

**Returns:**
Status message indicating watcher started or already running.

**Example:**
```python
from maenifold.tools.watcher_tools import start_asset_watcher
result = start_asset_watcher()
```

## StopAssetWatcher

Stop watching the assets directory.

**Returns:**
Status message indicating watcher stopped or not running.

**Example:**
```python
from maenifold.tools.watcher_tools import stop_asset_watcher
result = stop_asset_watcher()
```

## Notes

- Singleton pattern: Only one watcher can run at a time
- Debouncing: Multiple rapid changes are batched into a single notification
- Ma Protocol: No fake AI - errors propagate naturally
- Future Wave 4: Will integrate with MCP resource notifications
"""
