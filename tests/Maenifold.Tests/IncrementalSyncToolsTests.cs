using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using Maenifold.Tools;
using Maenifold.Utils;
using Microsoft.Data.Sqlite;
using NUnit.Framework;

namespace Maenifold.Tests;

public class IncrementalSyncToolsTests
{
    private MethodInfo _createdHandler = null!;
    private MethodInfo _changedHandler = null!;
    private MethodInfo _deletedHandler = null!;
    private MethodInfo _errorHandler = null!;

    [OneTimeSetUp]
    public void OneTimeSetUp()
    {
        Config.EnsureDirectories();
        GraphDatabase.InitializeDatabase();

        var type = typeof(IncrementalSyncTools);
        _createdHandler = type.GetMethod("ProcessFileCreated", BindingFlags.Static | BindingFlags.NonPublic)
            ?? throw new InvalidOperationException("ProcessFileCreated handler not found.");
        _changedHandler = type.GetMethod("ProcessFileChanged", BindingFlags.Static | BindingFlags.NonPublic)
            ?? throw new InvalidOperationException("ProcessFileChanged handler not found.");
        _deletedHandler = type.GetMethod("ProcessFileDeleted", BindingFlags.Static | BindingFlags.NonPublic)
            ?? throw new InvalidOperationException("ProcessFileDeleted handler not found.");
        _errorHandler = type.GetMethod("OnWatcherError", BindingFlags.Static | BindingFlags.NonPublic)
            ?? throw new InvalidOperationException("OnWatcherError handler not found.");
    }

    [SetUp]
    public void SetUp()
    {
        // Verify test isolation - ensure we're using test database
        Console.WriteLine($"[TEST SETUP] Using database: {Config.DatabasePath}");
        Console.WriteLine($"[TEST SETUP] Using memory path: {Config.MemoryPath}");

        // Ensure test database exists
        GraphDatabase.InitializeDatabase();

        // Clear any existing test data to ensure clean state
        try
        {
            using var conn = new SqliteConnection($"Data Source={Config.DatabasePath}");
            conn.OpenWithWAL();
            conn.Execute("DELETE FROM concept_mentions WHERE source_file LIKE 'memory://tests/%'");
            conn.Execute("DELETE FROM concept_graph_files WHERE source_file LIKE 'memory://tests/%'");
            conn.Execute("DELETE FROM concept_graph WHERE concept_a IN (SELECT concept_name FROM concepts WHERE concept_name LIKE '%test%' OR concept_name LIKE '%incremental%' OR concept_name LIKE '%watcher%')");
            conn.Execute("DELETE FROM file_content WHERE file_path LIKE 'memory://tests/%'");
            conn.Execute("DELETE FROM concepts WHERE concept_name LIKE '%test%' OR concept_name LIKE '%incremental%' OR concept_name LIKE '%watcher%'");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"[TEST SETUP WARNING] Failed to clean test data: {ex.Message}");
            // Re-initialize if needed
            GraphDatabase.InitializeDatabase();
        }
    }



    [Test]
    public void WatcherErrorsAreLogged()
    {
        // Note: This test doesn't use the database, so we don't need SetUp to run
        // We're only testing error logging functionality

        // Arrange: Capture Console.Error output
        var originalError = Console.Error;
        var capturedError = new StringWriter();
        Console.SetError(capturedError);

        try
        {
            // Arrange: Create test exception and ErrorEventArgs
            var testException = new InvalidOperationException("Test watcher error");
            var errorEventArgs = new ErrorEventArgs(testException);

            // Act: Invoke the error handler
            _errorHandler.Invoke(null, new object[] { null!, errorEventArgs });

            // Assert: Verify error was logged
            var errorOutput = capturedError.ToString();
            Assert.That(errorOutput, Does.Contain("[FileWatcher] Error: Test watcher error"),
                "Watcher errors should be logged to Console.Error with proper prefix");
        }
        finally
        {
            // Cleanup: Restore original Console.Error
            Console.SetError(originalError);
        }
    }

    private static void InvokeHandler(MethodInfo handler, string path)
    {
        handler.Invoke(null, new object[] { path });
    }

    private static (string a, string b) GetConceptPair(string conceptA, string conceptB)
    {
        return string.CompareOrdinal(conceptA, conceptB) < 0
            ? (conceptA, conceptB)
            : (conceptB, conceptA);
    }
}
