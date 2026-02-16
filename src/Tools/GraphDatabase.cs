using Microsoft.Data.Sqlite;
using Maenifold.Utils;

namespace Maenifold.Tools;

public static class GraphDatabase
{
    public static void InitializeDatabase()
    {
        using var conn = new SqliteConnection(Config.DatabaseConnectionString);
        conn.OpenWithWAL();
        conn.Execute("PRAGMA foreign_keys=ON");


        conn.Execute("PRAGMA journal_mode=WAL");
        conn.Execute("PRAGMA synchronous=NORMAL");

        CreateConceptTables(conn);
        CreateFileTables(conn);
        CreateGraphTables(conn);
        CreateVectorTables(conn);
        AddMissingColumns(conn);
    }

    private static void CreateConceptTables(SqliteConnection conn)
    {
        conn.Execute(@"
            CREATE TABLE IF NOT EXISTS concepts (
                concept_name TEXT PRIMARY KEY,
                first_seen TEXT,
                occurrence_count INTEGER DEFAULT 0
            )");

        conn.Execute(@"
            CREATE TABLE IF NOT EXISTS concept_mentions (
                concept_name TEXT NOT NULL,
                source_file TEXT NOT NULL,
                mention_count INTEGER DEFAULT 1,
                PRIMARY KEY (concept_name, source_file),
                FOREIGN KEY (concept_name) REFERENCES concepts(concept_name) ON DELETE CASCADE
            )");

        conn.Execute("CREATE INDEX IF NOT EXISTS idx_mentions_by_file ON concept_mentions(source_file)");
    }

    private static void CreateFileTables(SqliteConnection conn)
    {
        conn.Execute(@"
            CREATE TABLE IF NOT EXISTS file_content (
                file_path TEXT PRIMARY KEY,
                title TEXT,
                content TEXT,
                summary TEXT,
                last_indexed TEXT,
                status TEXT,
                file_md5 TEXT,
                file_size INTEGER
            )");

        conn.Execute(@"
            CREATE VIRTUAL TABLE IF NOT EXISTS file_search USING fts5(
                title,
                content,
                summary,
                content=file_content,
                content_rowid=rowid,
                tokenize='unicode61 remove_diacritics 0'
            )");

        // Triggers to keep FTS5 in sync with file_content
        conn.Execute(@"
            CREATE TRIGGER IF NOT EXISTS file_content_ai AFTER INSERT ON file_content BEGIN
                INSERT INTO file_search(rowid, title, content, summary)
                VALUES (new.rowid, new.title, new.content, new.summary);
            END");

        conn.Execute(@"
            CREATE TRIGGER IF NOT EXISTS file_content_ad AFTER DELETE ON file_content BEGIN
                INSERT INTO file_search(file_search, rowid, title, content, summary)
                VALUES('delete', old.rowid, old.title, old.content, old.summary);
            END");

        conn.Execute(@"
            CREATE TRIGGER IF NOT EXISTS file_content_au AFTER UPDATE ON file_content BEGIN
                INSERT INTO file_search(file_search, rowid, title, content, summary)
                VALUES('delete', old.rowid, old.title, old.content, old.summary);
                INSERT INTO file_search(rowid, title, content, summary)
                VALUES (new.rowid, new.title, new.content, new.summary);
            END");
    }

    private static void CreateGraphTables(SqliteConnection conn)
    {
        conn.Execute(@"
            CREATE TABLE IF NOT EXISTS concept_graph (
                concept_a TEXT NOT NULL,
                concept_b TEXT NOT NULL,
                co_occurrence_count INTEGER,
                source_files TEXT,
                PRIMARY KEY (concept_a, concept_b),
                FOREIGN KEY (concept_a) REFERENCES concepts(concept_name) ON DELETE CASCADE,
                FOREIGN KEY (concept_b) REFERENCES concepts(concept_name) ON DELETE CASCADE
            )");

        conn.Execute("CREATE INDEX IF NOT EXISTS idx_graph_concept_b ON concept_graph(concept_b)");
    }

    private static void CreateVectorTables(SqliteConnection conn)
    {
        try
        {

            conn.LoadVectorExtension();


            conn.Execute(@"
                CREATE VIRTUAL TABLE IF NOT EXISTS vec_concepts 
                USING vec0(concept_name TEXT PRIMARY KEY, embedding FLOAT[384])");


            conn.Execute(@"
                CREATE VIRTUAL TABLE IF NOT EXISTS vec_memory_files 
                USING vec0(file_path TEXT PRIMARY KEY, embedding FLOAT[384])");
        }
        catch (Exception ex)
        {

            Console.Error.WriteLine($"Warning: Failed to create vector tables: {ex.Message}");
        }
    }

    private static void AddMissingColumns(SqliteConnection conn)
    {

        try
        {
            conn.Execute("ALTER TABLE file_content ADD COLUMN status TEXT");
        }
        catch
        {
            // Column already exists
        }


        try
        {
            conn.Execute("ALTER TABLE file_content ADD COLUMN file_md5 TEXT");
        }
        catch
        {
            // Column already exists
        }

        // T-SYNC-MTIME-001.2: RTM FR-14.4 - Persist file_size for size guard
        try
        {
            conn.Execute("ALTER TABLE file_content ADD COLUMN file_size INTEGER");
        }
        catch
        {
            // Column already exists
        }

        // T-GRAPH-DECAY-002: RTM NFR-7.6.1 - Add last_accessed column for access-boosting decay behavior
        try
        {
            conn.Execute("ALTER TABLE file_content ADD COLUMN last_accessed TEXT");
        }
        catch
        {
            // Column already exists
        }

        // T-SYNC-UNIFY-001: concept_graph.source_files tracks which files contribute to each edge
        try
        {
            conn.Execute("ALTER TABLE concept_graph ADD COLUMN source_files TEXT");
        }
        catch
        {
            // Column already exists
        }
    }
}
