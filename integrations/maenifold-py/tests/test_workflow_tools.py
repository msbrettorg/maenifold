"""
Tests for workflow_tools module.

Tests workflow orchestration including:
- Starting workflows
- Session management
- Queue operations
- Concept validation
"""

import json
import os
import tempfile
from pathlib import Path

import pytest

from maenifold.config import Config
from maenifold.tools.workflow_tools import (
    _validate_concepts,
    session_exists,
    workflow,
)


@pytest.fixture
def temp_maenifold_root():
    """Create temporary maenifold root for testing."""
    with tempfile.TemporaryDirectory() as temp_dir:
        # Override config
        Config.override_root(temp_dir)

        # Create required directories
        os.makedirs(os.path.join(temp_dir, "memory", "thinking", "workflow"), exist_ok=True)
        os.makedirs(os.path.join(temp_dir, "assets", "workflows"), exist_ok=True)
        os.makedirs(os.path.join(temp_dir, "assets", "usage", "tools"), exist_ok=True)

        # Copy test workflow
        test_workflow = {
            "id": "test-workflow",
            "name": "Test Workflow",
            "description": "Simple test workflow",
            "steps": [
                {"id": 1, "name": "Step 1", "prompt": "Do step 1"},
                {"id": 2, "name": "Step 2", "prompt": "Do step 2"}
            ]
        }

        workflow_path = os.path.join(temp_dir, "assets", "workflows", "test-workflow.json")
        with open(workflow_path, 'w') as f:
            json.dump(test_workflow, f)

        yield temp_dir

        # Reset config
        Config.reset_overrides()


class TestConceptValidation:
    """Tests for concept validation."""

    def test_validate_concepts_with_response_concepts(self):
        """Should pass when response contains concepts."""
        result = _validate_concepts("Implementing [[REST API]] design", None)
        assert result is None

    def test_validate_concepts_with_thoughts_concepts(self):
        """Should pass when thoughts contain concepts."""
        result = _validate_concepts("Some response", "Considering [[architecture]] patterns")
        assert result is None

    def test_validate_concepts_without_concepts(self):
        """Should fail when neither response nor thoughts contain concepts."""
        result = _validate_concepts("Some response", "Some thoughts")
        assert result is not None
        assert "ERROR:" in result
        assert "[[concept]]" in result


class TestWorkflowStart:
    """Tests for starting workflows."""

    def test_start_workflow_success(self, temp_maenifold_root):
        """Should successfully start a new workflow."""
        result = workflow(workflow_id="test-workflow")

        assert "Created session:" in result
        assert "workflow-" in result
        assert "Step 1/2" in result

        # Extract session ID
        session_id = result.split("Created session: ")[1].split("\n")[0]
        assert session_exists("workflow", session_id)

    def test_start_workflow_not_found(self, temp_maenifold_root):
        """Should fail when workflow doesn't exist."""
        result = workflow(workflow_id="nonexistent")

        assert "ERROR:" in result
        assert "not found" in result

    def test_start_workflow_array(self, temp_maenifold_root):
        """Should support starting workflow queue with array."""
        result = workflow(workflow_id='["test-workflow"]')

        assert "Created session:" in result
        assert "Step 1/2" in result


class TestWorkflowContinue:
    """Tests for continuing workflows."""

    def test_continue_workflow_success(self, temp_maenifold_root):
        """Should successfully continue workflow with response."""
        # Start workflow
        start_result = workflow(workflow_id="test-workflow")
        session_id = start_result.split("Created session: ")[1].split("\n")[0]

        # Continue with response
        continue_result = workflow(
            session_id=session_id,
            response="Completed [[first-step]] successfully"
        )

        assert "Step 2/2" in continue_result

    def test_continue_workflow_missing_concepts(self, temp_maenifold_root):
        """Should fail when response lacks concepts."""
        # Start workflow
        start_result = workflow(workflow_id="test-workflow")
        session_id = start_result.split("Created session: ")[1].split("\n")[0]

        # Continue without concepts
        continue_result = workflow(
            session_id=session_id,
            response="Completed first step"
        )

        assert "ERROR:" in continue_result
        assert "[[concept]]" in continue_result

    def test_continue_workflow_with_thoughts(self, temp_maenifold_root):
        """Should accept concepts in thoughts field."""
        # Start workflow
        start_result = workflow(workflow_id="test-workflow")
        session_id = start_result.split("Created session: ")[1].split("\n")[0]

        # Continue with concepts in thoughts
        continue_result = workflow(
            session_id=session_id,
            response="Completed step",
            thoughts="Using [[test-pattern]] approach"
        )

        assert "Step 2/2" in continue_result


class TestWorkflowCompletion:
    """Tests for completing workflows."""

    def test_complete_workflow_success(self, temp_maenifold_root):
        """Should successfully complete workflow with conclusion."""
        # Start workflow
        start_result = workflow(workflow_id="test-workflow")
        session_id = start_result.split("Created session: ")[1].split("\n")[0]

        # Complete step 1
        workflow(session_id=session_id, response="Step 1 using [[pattern-a]]")

        # Complete step 2 with conclusion
        result = workflow(
            session_id=session_id,
            response="Step 2 using [[pattern-b]]",
            status="completed",
            conclusion="Successfully completed [[test-workflow]]"
        )

        assert "✅" in result
        assert "completed" in result

    def test_complete_workflow_missing_conclusion(self, temp_maenifold_root):
        """Should fail when completion lacks conclusion."""
        # Start workflow
        start_result = workflow(workflow_id="test-workflow")
        session_id = start_result.split("Created session: ")[1].split("\n")[0]

        # Try to complete without conclusion
        result = workflow(
            session_id=session_id,
            response="Step 1 using [[pattern-a]]",
            status="completed"
        )

        assert "ERROR:" in result
        assert "Conclusion required" in result


class TestWorkflowView:
    """Tests for viewing workflow status."""

    def test_view_workflow_success(self, temp_maenifold_root):
        """Should display workflow queue status."""
        # Start workflow
        start_result = workflow(workflow_id="test-workflow")
        session_id = start_result.split("Created session: ")[1].split("\n")[0]

        # View status
        result = workflow(session_id=session_id, view=True)

        assert "Queue:" in result
        assert "test-workflow" in result
        assert "Position:" in result


class TestWorkflowAppend:
    """Tests for appending to workflow queue."""

    def test_append_workflow_success(self, temp_maenifold_root):
        """Should successfully append workflow to queue."""
        # Start workflow
        start_result = workflow(workflow_id="test-workflow")
        session_id = start_result.split("Created session: ")[1].split("\n")[0]

        # Append another workflow
        result = workflow(session_id=session_id, append="test-workflow")

        assert "Added" in result
        assert "to queue" in result


class TestParameterValidation:
    """Tests for parameter validation."""

    def test_both_workflow_id_and_session_id(self, temp_maenifold_root):
        """Should fail when both workflow_id and session_id provided."""
        result = workflow(workflow_id="test-workflow", session_id="workflow-12345")

        assert "ERROR:" in result
        assert "Cannot provide both" in result

    def test_invalid_operation(self, temp_maenifold_root):
        """Should fail with invalid parameter combination."""
        result = workflow()

        assert "ERROR:" in result
        assert "Invalid parameter combination" in result
