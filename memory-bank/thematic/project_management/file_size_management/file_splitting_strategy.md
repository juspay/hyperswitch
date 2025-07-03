# File Splitting Strategy

This document outlines comprehensive strategies for splitting large Memory Bank documentation files into smaller, more manageable units. It provides concrete approaches tailored to different types of documentation and content structures, with examples based on actual files identified in the size analysis.

## Strategic Approaches

The Memory Bank documentation supports three primary splitting strategies, each suited to different content types and organizational needs:

### 1. Topic-Based Splitting

**Best for:**
- Files covering multiple distinct topics
- Documentation with clearly separable subject areas
- Content that different users might need to access independently

**Implementation Structure:**
```
/original-topic/
├── overview.md                  # Summary and links to detailed topics
├── topic1.md                    # First major topic
├── topic2.md                    # Second major topic
└── topic3.md                    # Third major topic
```

**Example from Analysis:** `crateIndex.md` (693 lines)
- This file contains listings of multiple crates and could be split by crate category
- Create an index file with links to separate files for each crate category

### 2. Hierarchical Splitting

**Best for:**
- Files with a clear parent-child information structure
- Documentation with a high-level overview and detailed subsections
- Content with varying levels of detail that different readers might need

**Implementation Structure:**
```
/subject-area/
├── overview.md              # High-level architecture and links
├── component1.md            # Detailed information on component 1
├── component2.md            # Detailed information on component 2
└── component3.md            # Detailed information on component 3
```

**Example from Analysis:** `thematic/crates/router/flows/payment_flows.md` (617 lines)
- This file documents payment flows and could be split hierarchically
- Create an overview file with links to separate files for each major flow stage

### 3. Temporal Splitting

**Best for:**
- Files containing historical/archival content mixed with current information
- Documentation that includes deprecated features alongside active ones
- Reference material where historical context is valuable but not frequently needed

**Implementation Structure:**
```
/feature-area/
├── current-implementation.md    # Current implementation details
└── /archive/
    └── deprecated-methods.md    # Historical methods no longer in use
```

**Example from Analysis:** `thematic/crates/hyperswitch_connectors/connector_implementation_guide.md` (702 lines)
- May contain implementation guidelines for both current and legacy connector patterns
- Split into current best practices and archived older approaches

## Implementation Guidelines by Content Type

### Flow Documentation

Files like `payment_flows.md` (617 lines), `refund_flows.md` (440 lines), and `webhook_flows.md` (421 lines) should be split using the following approach:

1. **Create Flow-Specific Directory**
   ```
   /flows/payments/
   ```

2. **Create Overview Document**
   - Include high-level flow description
   - Provide diagram of the complete flow
   - Include links to detailed stage documentation

3. **Split by Flow Stages**
   ```
   /flows/payments/
   ├── overview.md                 # High-level flow and diagram
   ├── initiation.md               # Payment initiation stage
   ├── processing.md               # Payment processing stage
   ├── completion.md               # Payment completion stage
   └── error-handling.md           # Error scenarios
   ```

4. **Cross-Reference Implementation**
   - Ensure each stage document links back to the overview
   - Include "Previous Stage" and "Next Stage" navigation links
   - Add "See Also" sections for related flows

### Connector Documentation

Files like `connector_implementation_guide.md` (702 lines) and `connector_testing_guide.md` (497 lines) should be split as follows:

1. **Create Connector Documentation Directory**
   ```
   /connectors/
   ```

2. **Implement Topic-Based Splitting**
   ```
   /connectors/
   ├── overview.md                  # General connector concepts
   ├── implementation/              # Implementation subdirectory
   │   ├── overview.md              # Implementation overview
   │   ├── basic-setup.md           # Basic setup steps
   │   ├── advanced-features.md     # Advanced implementation
   │   └── best-practices.md        # Implementation best practices
   ├── testing/                     # Testing subdirectory
   │   ├── overview.md              # Testing overview
   │   ├── unit-tests.md            # Unit testing guidelines
   │   ├── integration-tests.md     # Integration testing
   │   └── mock-services.md         # Using mock services
   └── configuration/               # Configuration subdirectory
       ├── overview.md              # Configuration overview
       ├── parameters.md            # Configuration parameters
       └── examples.md              # Configuration examples
   ```

3. **Handle Cross-References**
   - Update all references to original files
   - Ensure consistent navigation between related documents

### Large Index Files

Files like `crateIndex.md` (693 lines) should be split using the following approach:

1. **Create Category-Based Structure**
   ```
   /indices/
   ├── crate-index.md               # Main index with categorization
   ├── core-crates.md               # Details on core crates
   ├── utility-crates.md            # Details on utility crates
   ├── connector-crates.md          # Details on connector crates
   └── supporting-crates.md         # Details on other crates
   ```

2. **Implement Alphabetical Splitting (Alternative)**
   ```
   /indices/
   ├── crate-index.md               # Main index with links to all sub-indices
   ├── crates-a-e.md                # Crates starting with A through E
   ├── crates-f-l.md                # Crates starting with F through L
   ├── crates-m-r.md                # Crates starting with M through R
   └── crates-s-z.md                # Crates starting with S through Z
   ```

## Naming Conventions

To ensure consistency and discoverability:

1. **Overview Files**
   - Always name the main entry point `overview.md`
   - Include the original file name or subject in the directory name

2. **Component Files**
   - Use kebab-case for all file names (`payment-initiation.md` not `paymentInitiation.md`)
   - Prefix with numbers if sequence is important (`01-setup.md`, `02-configuration.md`)
   - Use descriptive, concise names focusing on the specific content

3. **Directories**
   - Use kebab-case for directory names
   - Structure directories to reflect logical content organization
   - Avoid deeply nested structures (no more than 3 levels deep)

## Metadata Headers

Each split file should include a consistent metadata header:

```markdown
---
title: Payment Initiation
parent: Payment Flows Overview
parent_path: ../overview.md
position: 2
last_updated: 2025-05-27
related_files:
  - ../processing.md
  - ../../webhooks/payment-webhooks.md
---
```

## Implementation Process

For each file that needs splitting:

1. **Analyze Content Structure**
   - Identify logical break points
   - Map relationships between content sections
   - Determine the most appropriate splitting strategy

2. **Create Directory Structure**
   - Create the appropriate directory structure
   - Use consistent naming conventions

3. **Create Overview File**
   - Extract high-level content for the overview
   - Add clear navigation links to all component files
   - Include a table of contents

4. **Split Content into Component Files**
   - Move detailed content to appropriate component files
   - Add metadata headers to each file
   - Ensure each file is independently readable
   - Add navigation links (previous/next/parent)

5. **Update Cross-References**
   - Identify and update all references to the original file
   - Ensure all internal links are updated

6. **Validate the Structure**
   - Verify all content is preserved
   - Check all links work correctly
   - Ensure navigation is intuitive

## Case Studies from Analysis Report

### Case Study 1: `payment_flows.md` (617 lines)

**Current Structure:**
- Large file with multiple payment flow descriptions
- Sections for different payment types and stages
- Diagrams and code examples throughout

**Recommended Splitting (Hierarchical):**
```
/flows/payments/
├── overview.md                 # Overview and main diagram
├── card-payments/              # Card payment flows
│   ├── overview.md             # Card payment overview
│   ├── authorization.md        # Authorization flow
│   ├── capture.md              # Capture flow
│   └── refund.md               # Refund flow for cards
├── bank-transfers/             # Bank transfer flows
│   ├── overview.md             # Bank transfer overview
│   ├── initiation.md           # Initiation flow
│   └── confirmation.md         # Confirmation flow
└── common/                     # Common components
    ├── error-handling.md       # Error handling for all flows
    └── webhooks.md             # Webhook integration points
```

### Case Study 2: `connector_implementation_guide.md` (702 lines)

**Current Structure:**
- Comprehensive guide for implementing connectors
- Sections on setup, configuration, testing, and deployment
- Code examples and troubleshooting guidance

**Recommended Splitting (Topic-Based):**
```
/connectors/implementation/
├── overview.md                 # Implementation overview
├── prerequisites.md            # Requirements and setup
├── basic-implementation.md     # Core implementation steps
├── advanced-features.md        # Advanced functionality
├── testing.md                  # Testing implementation
├── deployment.md               # Deployment guidelines
└── troubleshooting.md          # Common issues and solutions
```

## Special Considerations

### API Documentation

For API documentation like `webhook_handling.md` (391 lines):

1. **Split by Endpoint Category**
   - Group related endpoints together
   - Create separate files for different API areas

2. **Consider Generated vs Manual Content**
   - If some content is auto-generated, keep it separate
   - Maintain manual annotations in their own files

### Configuration Documentation

For configuration files like `router_configuration.md` (344 lines):

1. **Split by Configuration Domain**
   - Separate basic configuration from advanced options
   - Group related configuration settings

2. **Use Tables for Quick Reference**
   - Create consolidated parameter tables in overview files
   - Provide detailed explanations in separate files

## Tooling Support

To assist with the splitting process:

1. **Header Level Adjustment**
   - Decrease heading levels in split files as appropriate
   - Ensure consistent heading hierarchy

2. **Link Verification**
   - Use link checking tools to verify all links work after splitting
   - Update any broken references

3. **Navigation Generation**
   - Generate consistent navigation sections for all split files
   - Include breadcrumb trails for easy navigation

## Related Documents

- [File Identification Criteria](file_identification_criteria.md)
- [File Size Management Guide](../file_size_management_guide.md)
- [Index Creation Process](./index_creation_process.md) (to be created)
- [Cross-Reference Update System](./cross_reference_update_system.md) (to be created)
