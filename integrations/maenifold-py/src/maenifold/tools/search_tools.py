"""
Memory search tools for discovering knowledge through full-text, semantic, and hybrid search.

This module provides:
- search_memories: Multi-mode search (Hybrid/Semantic/FullText) with pagination
- find_similar_concepts: Semantic concept discovery using vector similarity
"""

import os
import sqlite3
from typing import List, Optional, Tuple

from maenifold.config import Config
from maenifold.utils.markdown_io import path_to_uri, read_markdown, uri_to_path
from maenifold.utils.sqlite_extensions import load_vector_extension, open_read_only, query as db_query
from maenifold.utils.vector_tools import VectorTools


# Stopwords for filtering query terms
STOPWORDS = {
    "a", "an", "and", "are", "as", "at", "be", "but", "by", "for", "if", "in", "into",
    "is", "it", "no", "not", "of", "on", "or", "such", "that", "the", "their", "then",
    "there", "these", "they", "this", "to", "was", "will", "with", "we", "you", "your",
    "our", "from"
}


def _get_informative_terms(query: str) -> List[str]:
    """
    Extract informative terms from query by filtering stopwords.

    Args:
        query: Search query string

    Returns:
        List of non-stopword terms in lowercase
    """
    if not query or not query.strip():
        return []

    terms = query.lower().split()
    return [term for term in terms if term and term not in STOPWORDS]


def _get_file_display_info(file_path_or_uri: str, query: str) -> Tuple[str, str]:
    """
    Get display title and snippet for a file.

    Args:
        file_path_or_uri: File path or memory:// URI
        query: Search query for snippet extraction

    Returns:
        Tuple of (title, snippet)
    """
    try:
        # Convert URI to path if needed
        if file_path_or_uri.startswith("memory://"):
            file_path = uri_to_path(file_path_or_uri, Config.memory_path)
        else:
            file_path = file_path_or_uri

        frontmatter, content, _ = read_markdown(file_path)

        # Extract title
        title = frontmatter.get("title", os.path.splitext(os.path.basename(file_path))[0]) if frontmatter else os.path.splitext(os.path.basename(file_path))[0]

        # Extract snippet (first 200 chars around query term)
        snippet = _extract_snippet(content, query, 200)

        return (title, snippet)
    except Exception:
        # Fall back to filename
        name = file_path_or_uri.replace("memory://", "").replace("/", os.sep) if file_path_or_uri.startswith("memory://") else file_path_or_uri
        return (os.path.splitext(os.path.basename(name))[0], "")


def _extract_snippet(content: str, query: str, max_length: int) -> str:
    """
    Extract snippet from content centered around query terms.

    Args:
        content: Full content text
        query: Search query
        max_length: Maximum snippet length

    Returns:
        Content snippet
    """
    if not content:
        return ""

    terms = _get_informative_terms(query)
    first_term_index = -1

    if terms:
        # Find position of first informative term
        for term in terms:
            idx = content.lower().find(term)
            if idx >= 0 and (first_term_index == -1 or idx < first_term_index):
                first_term_index = idx

    if first_term_index < 0:
        first_term_index = 0

    # Extract snippet centered around the first term
    start = max(0, first_term_index - 20)
    length = min(max_length, len(content) - start)
    snippet = content[start:start + length].replace("\r\n", " ").replace("\n", " ").strip()

    return snippet


def _perform_fulltext_search(
    query: str,
    search_path: str,
    page_size: int,
    page: int,
    tags: Optional[List[str]],
    min_score: float
) -> str:
    """
    Perform full-text search using FTS5.

    Args:
        query: Search query
        search_path: Directory path to search
        page_size: Results per page
        page: Page number
        tags: Optional tag filters
        min_score: Minimum score threshold

    Returns:
        Formatted search results
    """
    if not _get_informative_terms(query):
        return (
            "ERROR: Query contains no informative keywords. "
            "Try adding specific terms (e.g. \"machine learning\", \"NLP\").\n\n"
            "**Suggested next steps:** SearchMemories, ReadMemory, BuildContext"
        )

    try:
        with open_read_only(Config.database_path) as conn:
            # Query FTS5 table
            sql = """
                SELECT fc.file_path, bm25(file_search) as score
                FROM file_search
                JOIN file_content fc ON file_search.rowid = fc.rowid
                WHERE file_search MATCH ?
                ORDER BY score
            """

            results = db_query(conn, sql, (query,))

            # Normalize scores (BM25 returns negative scores, lower = better)
            if results:
                max_score = abs(min(r[1] for r in results))
                if max_score > 0:
                    normalized = [(r[0], abs(r[1]) / max_score) for r in results]
                else:
                    normalized = [(r[0], 1.0) for r in results]
            else:
                normalized = []

            # Filter by tags if specified
            if tags:
                filtered = []
                for file_uri, score in normalized:
                    if _file_has_tags(file_uri, tags):
                        filtered.append((file_uri, score))
                normalized = filtered

            # Filter by min_score
            filtered_results = [(uri, score) for uri, score in normalized if score >= min_score]

            # Get display info
            results_with_info = []
            for file_uri, score in filtered_results:
                title, snippet = _get_file_display_info(file_uri, query)
                results_with_info.append((file_uri, score, title, snippet))

            # Paginate
            total_results = len(results_with_info)
            start_idx = (page - 1) * page_size
            end_idx = start_idx + page_size
            paginated = results_with_info[start_idx:end_idx]

            # Format output
            lines = [
                "📝 **Full-Text Search** (keyword matching)",
                f"Found {total_results} matches (page {page}):\n"
            ]

            for file_uri, score, title, snippet in paginated:
                lines.append(f"📄 **{title}** ({file_uri})")
                lines.append(f"   📊 Text relevance: {score:.2f}")
                lines.append(f"   📝 {snippet}")
                lines.append("")

            return "\n".join(lines) + "\n**Suggested next steps:** ReadMemory, BuildContext"

    except Exception as e:
        return f"Error performing full-text search: {e}\n\n**Suggested next steps:** Sync, SearchMemories"


def _perform_semantic_search(
    query: str,
    search_path: str,
    page_size: int,
    page: int,
    tags: Optional[List[str]],
    min_score: float
) -> str:
    """
    Perform semantic search using vector embeddings.

    Args:
        query: Search query
        search_path: Directory path to search
        page_size: Results per page
        page: Page number
        tags: Optional tag filters
        min_score: Minimum score threshold

    Returns:
        Formatted search results
    """
    if not _get_informative_terms(query):
        return (
            "ERROR: Query contains no informative keywords. "
            "Try adding specific terms (e.g. \"machine learning\", \"NLP\").\n\n"
            "**Suggested next steps:** SearchMemories, ReadMemory, BuildContext"
        )

    try:
        # Generate query embedding
        query_embedding = VectorTools.generate_embedding(query)
        embedding_blob = VectorTools.to_sqlite_vector_blob(query_embedding)

        if not embedding_blob:
            return "Unable to generate embedding for query.\n\n**Suggested next steps:** SearchMemories, BuildContext"

        with open_read_only(Config.database_path) as conn:
            load_vector_extension(conn)

            # Determine folder filter
            folder_uri_prefix = None
            if search_path != Config.memory_path:
                rel_folder = os.path.relpath(search_path, Config.memory_path)
                if rel_folder and rel_folder != ".":
                    folder_uri_prefix = f"memory://{rel_folder.replace(os.sep, '/')}/"

            # Query vector table
            sql = """
                SELECT file_path, vec_distance_cosine(embedding, ?) as distance
                FROM vec_memory_files
                WHERE embedding IS NOT NULL
            """

            params = [embedding_blob]

            if folder_uri_prefix:
                sql += " AND file_path LIKE ?"
                params.append(folder_uri_prefix + "%")

            sql += " ORDER BY distance LIMIT ?"
            params.append(page_size * 2)

            results = db_query(conn, sql, params)

            # Normalize distances to similarity scores (0-1, higher = better)
            if results:
                distances = [r[1] for r in results if r[1] is not None]
                if distances:
                    min_dist = min(distances)
                    max_dist = max(distances)
                    if abs(max_dist - min_dist) > 1e-12:
                        normalized = [(r[0], 1.0 - (r[1] - min_dist) / (max_dist - min_dist)) for r in results if r[1] is not None]
                    else:
                        normalized = [(r[0], 1.0) for r in results if r[1] is not None]
                else:
                    normalized = []
            else:
                normalized = []

            # Filter by tags if specified
            if tags:
                filtered = []
                for file_uri, score in normalized:
                    if _file_has_tags(file_uri, tags):
                        filtered.append((file_uri, score))
                normalized = filtered

            # Filter by min_score
            filtered_results = [(uri, score) for uri, score in normalized if score >= min_score]

            # Get display info
            results_with_info = []
            for file_uri, score in filtered_results:
                title, snippet = _get_file_display_info(file_uri, query)
                results_with_info.append((file_uri, score, title, snippet))

            # Paginate
            total_results = len(results_with_info)
            start_idx = (page - 1) * page_size
            end_idx = start_idx + page_size
            paginated = results_with_info[start_idx:end_idx]

            # Format output
            lines = [
                "🧠 **Semantic Search** (concept similarity)",
                f"Found {total_results} matches (page {page}):\n"
            ]

            for file_uri, score, title, snippet in paginated:
                lines.append(f"📄 **{title}** ({file_uri})")
                lines.append(f"   🎯 Semantic similarity: {score:.3f}")
                lines.append(f"   📝 {snippet}")
                lines.append("")

            return "\n".join(lines) + "\n**Suggested next steps:** ReadMemory, BuildContext, FindSimilarConcepts"

    except Exception as e:
        return f"Error performing semantic search: {e}\n\n**Suggested next steps:** Sync, SearchMemories"


def _apply_rrf(
    vector_results: List[Tuple[str, float]],
    text_results: List[Tuple[str, float]],
    query: str,
    k: int = 60
) -> List[Tuple[str, float, float, float, str, str]]:
    """
    Apply Reciprocal Rank Fusion to combine search results.

    Args:
        vector_results: Semantic search results (uri, score)
        text_results: Full-text search results (uri, score)
        query: Original query for snippet extraction
        k: RRF constant (default 60)

    Returns:
        List of (uri, fused_score, text_score, semantic_score, title, snippet)
    """
    rrf_scores = {}
    original_scores = {}
    file_info = {}

    # Process vector results
    for i, (file_uri, score) in enumerate(vector_results):
        rrf_score = 1.0 / (k + i + 1)
        rrf_scores[file_uri] = rrf_scores.get(file_uri, 0.0) + rrf_score
        original_scores[file_uri] = (0.0, score)

        if file_uri not in file_info:
            title, snippet = _get_file_display_info(file_uri, query)
            file_info[file_uri] = (title, snippet)

    # Process text results
    for i, (file_uri, score) in enumerate(text_results):
        rrf_score = 1.0 / (k + i + 1)
        rrf_scores[file_uri] = rrf_scores.get(file_uri, 0.0) + rrf_score

        # Normalize text score to 0-1
        normalized_text_score = min(score / 100.0, 1.0)
        if file_uri in original_scores:
            original_scores[file_uri] = (normalized_text_score, original_scores[file_uri][1])
        else:
            original_scores[file_uri] = (normalized_text_score, 0.0)

        if file_uri not in file_info:
            title, snippet = _get_file_display_info(file_uri, query)
            file_info[file_uri] = (title, snippet)

    # Build fused results
    fused = []
    for file_uri, fused_score in sorted(rrf_scores.items(), key=lambda x: x[1], reverse=True):
        text_score, semantic_score = original_scores.get(file_uri, (0.0, 0.0))
        title, snippet = file_info.get(file_uri, (os.path.splitext(os.path.basename(file_uri))[0], ""))
        fused.append((file_uri, fused_score, text_score, semantic_score, title, snippet))

    return fused


def _perform_hybrid_search(
    query: str,
    search_path: str,
    page_size: int,
    page: int,
    tags: Optional[List[str]],
    min_score: float
) -> str:
    """
    Perform hybrid search combining full-text and semantic search using RRF.

    Args:
        query: Search query
        search_path: Directory path to search
        page_size: Results per page
        page: Page number
        tags: Optional tag filters
        min_score: Minimum score threshold

    Returns:
        Formatted search results
    """
    if not _get_informative_terms(query):
        return (
            "ERROR: Query contains no informative keywords. "
            "Try adding specific terms (e.g. \"machine learning\", \"NLP\").\n\n"
            "**Suggested next steps:** SearchMemories, ReadMemory, BuildContext"
        )

    try:
        # Get vector results
        vector_results = _get_vector_results(query, search_path, page_size * 3, tags)

        # Get text results
        text_results = _get_text_results(query, search_path, page_size * 3, tags)

        # Apply RRF
        merged = _apply_rrf(vector_results, text_results, query, k=60)

        # Filter by min_score
        filtered_results = [r for r in merged if r[1] >= min_score]

        # Paginate
        total_results = len(filtered_results)
        start_idx = (page - 1) * page_size
        end_idx = start_idx + page_size
        paginated = filtered_results[start_idx:end_idx]

        # Format output
        lines = [
            "🔍 **Hybrid Search** (combining text + semantic similarity)",
            f"Found {total_results} matches (page {page}):",
            f"Text results: {len(text_results)}, Semantic results: {len(vector_results)}\n"
        ]

        for file_uri, fused_score, text_score, semantic_score, title, snippet in paginated:
            lines.append(f"📄 **{title}** ({file_uri})")
            lines.append(f"   📊 Scores - Fused: {fused_score:.3f} | Text: {text_score:.3f} | Semantic: {semantic_score:.3f}")
            lines.append(f"   📝 {snippet}")
            lines.append("")

        return "\n".join(lines) + "\n**Suggested next steps:** ReadMemory, BuildContext"

    except Exception as e:
        return f"Error performing hybrid search: {e}\n\n**Suggested next steps:** Sync, SearchMemories"


def _get_vector_results(
    query: str,
    search_path: str,
    max_results: int,
    tags: Optional[List[str]]
) -> List[Tuple[str, float]]:
    """Get vector search results."""
    try:
        query_embedding = VectorTools.generate_embedding(query)
        embedding_blob = VectorTools.to_sqlite_vector_blob(query_embedding)

        if not embedding_blob:
            return []

        with open_read_only(Config.database_path) as conn:
            load_vector_extension(conn)

            folder_uri_prefix = None
            if search_path != Config.memory_path:
                rel_folder = os.path.relpath(search_path, Config.memory_path)
                if rel_folder and rel_folder != ".":
                    folder_uri_prefix = f"memory://{rel_folder.replace(os.sep, '/')}/"

            sql = """
                SELECT file_path, vec_distance_cosine(embedding, ?) as distance
                FROM vec_memory_files
                WHERE embedding IS NOT NULL
            """

            params = [embedding_blob]

            if folder_uri_prefix:
                sql += " AND file_path LIKE ?"
                params.append(folder_uri_prefix + "%")

            sql += " ORDER BY distance LIMIT ?"
            params.append(max_results)

            results = db_query(conn, sql, params)

            # Normalize to 0-1 scores
            if results:
                distances = [r[1] for r in results if r[1] is not None]
                if distances:
                    min_dist = min(distances)
                    max_dist = max(distances)
                    if abs(max_dist - min_dist) > 1e-12:
                        normalized = [(r[0], 1.0 - (r[1] - min_dist) / (max_dist - min_dist)) for r in results if r[1] is not None]
                    else:
                        normalized = [(r[0], 1.0) for r in results if r[1] is not None]
                else:
                    normalized = []
            else:
                normalized = []

            # Filter by tags
            if tags:
                filtered = []
                for file_uri, score in normalized:
                    if _file_has_tags(file_uri, tags):
                        filtered.append((file_uri, score))
                return filtered

            return normalized

    except Exception:
        return []


def _get_text_results(
    query: str,
    search_path: str,
    max_results: int,
    tags: Optional[List[str]]
) -> List[Tuple[str, float]]:
    """Get full-text search results."""
    try:
        with open_read_only(Config.database_path) as conn:
            sql = """
                SELECT fc.file_path, bm25(file_search) as score
                FROM file_search
                JOIN file_content fc ON file_search.rowid = fc.rowid
                WHERE file_search MATCH ?
                ORDER BY score
                LIMIT ?
            """

            results = db_query(conn, sql, (query, max_results))

            # Normalize scores
            if results:
                max_score = abs(min(r[1] for r in results))
                if max_score > 0:
                    normalized = [(r[0], abs(r[1]) / max_score) for r in results]
                else:
                    normalized = [(r[0], 1.0) for r in results]
            else:
                normalized = []

            # Filter by tags
            if tags:
                filtered = []
                for file_uri, score in normalized:
                    if _file_has_tags(file_uri, tags):
                        filtered.append((file_uri, score))
                return filtered

            return normalized

    except Exception:
        return []


def _file_has_tags(file_uri: str, required_tags: List[str]) -> bool:
    """Check if file has all required tags."""
    try:
        file_path = uri_to_path(file_uri, Config.memory_path)
        frontmatter, _, _ = read_markdown(file_path)

        if not frontmatter or "tags" not in frontmatter:
            return False

        file_tags = frontmatter["tags"]
        if not isinstance(file_tags, list):
            return False

        file_tags_lower = [str(t).lower() for t in file_tags]
        return all(tag.lower() in file_tags_lower for tag in required_tags)

    except Exception:
        return False


def search_memories(
    query: str,
    mode: str = "Hybrid",
    page_size: int = 10,
    page: int = 1,
    folder: Optional[str] = None,
    tags: Optional[List[str]] = None,
    min_score: float = 0.0,
    learn: bool = False
) -> str:
    """
    Discover existing knowledge files through flexible search modes.

    Supports three search modes:
    - Hybrid (default): Combines text and semantic search using RRF
    - Semantic: Vector similarity search using embeddings
    - FullText: Keyword matching using FTS5

    Args:
        query: Search query (looks in file contents)
        mode: Search mode ("Hybrid", "Semantic", or "FullText")
        page_size: Maximum files to return (default 10)
        page: Page number for results (default 1)
        folder: Optional folder path filter
        tags: Optional list of required tags
        min_score: Minimum similarity score threshold 0.0-1.0 (default 0.0)
        learn: Return help documentation instead of executing (default False)

    Returns:
        Formatted search results with scores and snippets
    """
    if learn:
        return "Help documentation for search_memories would be loaded here."

    search_path = os.path.join(Config.memory_path, folder) if folder else Config.memory_path

    if not os.path.exists(search_path):
        return "No memories found in specified folder"

    mode_lower = mode.lower()

    if mode_lower == "semantic":
        return _perform_semantic_search(query, search_path, page_size, page, tags, min_score)
    elif mode_lower == "fulltext":
        return _perform_fulltext_search(query, search_path, page_size, page, tags, min_score)
    else:  # hybrid (default)
        return _perform_hybrid_search(query, search_path, page_size, page, tags, min_score)


def find_similar_concepts(
    concept_name: str,
    max_results: int = 10,
    learn: bool = False
) -> str:
    """
    Find concepts similar to input using vector embeddings.

    Uses 384-dimensional embeddings with cosine similarity to discover
    semantically related concepts in the knowledge graph.

    Args:
        concept_name: Concept to find similar concepts for
        max_results: Maximum number of similar concepts to return (default 10)
        learn: Return help documentation instead of executing (default False)

    Returns:
        Ranked list of similar concepts with similarity scores
    """
    if learn:
        return "Help documentation for find_similar_concepts would be loaded here."

    if not concept_name or not concept_name.strip():
        return "ERROR: conceptName parameter is required and cannot be empty"

    try:
        # Normalize concept name (lowercase-with-hyphens)
        from maenifold.utils.markdown_io import normalize_concept
        concept_name = normalize_concept(concept_name)

        # Generate embedding
        query_embedding = VectorTools.generate_embedding(concept_name)
        embedding_blob = VectorTools.to_sqlite_vector_blob(query_embedding)

        if not embedding_blob:
            return f"Unable to generate embedding for '{concept_name}'."

        with open_read_only(Config.database_path) as conn:
            load_vector_extension(conn)

            sql = """
                SELECT concept_name, vec_distance_cosine(embedding, ?) as distance
                FROM vec_concepts
                WHERE concept_name != ? AND concept_name != ''
                ORDER BY distance
                LIMIT ?
            """

            results = db_query(conn, sql, (embedding_blob, concept_name, max_results))

        if not results:
            return (
                f"No similar concepts found for '{concept_name}'. "
                "Run Sync to ensure concept embeddings are generated."
            )

        # Convert distances to similarity scores
        similar_concepts = []
        for row in results:
            concept = row[0]
            distance = row[1]
            # Convert distance to similarity: similarity = 1 / (1 + distance)
            similarity = 1.0 / (1.0 + max(0, distance))
            similar_concepts.append((concept, similarity))

        # Format output
        lines = [f"Similar concepts to '{concept_name}' (by semantic similarity):\n"]

        for concept, similarity in similar_concepts:
            lines.append(f"  • {concept} (similarity: {similarity:.3f})")

        return "\n".join(lines) + "\n\n**Suggested next steps:** BuildContext, SearchMemories"

    except Exception as e:
        return (
            f"Error finding similar concepts: {e}. "
            "Ensure vector extension is available and run Sync to generate concept embeddings."
        )
