# Index Creation Process

This document outlines the process for creating and maintaining index files for split documentation in the Memory Bank. Index files serve as entry points and navigation aids for documentation that has been split across multiple files.

## Purpose of Index Files

Index files serve several critical functions in split documentation:

1. **Entry Point** - Provide a consistent starting point for documentation topics
2. **Navigation Hub** - Offer links to all component documents
3. **Overview** - Present high-level summaries of the topic
4. **Organization** - Structure the content in a logical manner
5. **Context** - Provide the relationships between component documents

## Types of Index Files

The Memory Bank documentation uses three primary types of index files:

### 1. Overview Index

The most common type, used as the main entry point for documentation topics that have been split into multiple files.

**Example Structure:**
```markdown
# Payment Flows Overview

This document provides an overview of payment flows in Hyperswitch and serves as an index for detailed documentation on specific payment processes.

## Introduction

[Brief introduction to payment flows and their importance]

## Payment Flow Types

Hyperswitch supports the following payment flow types:

- [Card Payments](./card-payments/overview.md) - Standard card processing flows
- [Bank Transfers](./bank-transfers/overview.md) - ACH and other bank transfer methods
- [Digital Wallets](./digital-wallets/overview.md) - Integration with popular wallet providers

## Common Components

These components are used across multiple payment flows:

- [Error Handling](./common/error-handling.md) - Standardized error handling
- [Webhook Integration](./common/webhooks.md) - Event-based notifications
```

### 2. Category Index

Used for organizing content by categories, especially useful for reference documentation.

**Example Structure:**
```markdown
# Crate Index

This document categorizes the crates in the Hyperswitch project by their function and purpose.

## Core Crates

These crates form the essential functionality of Hyperswitch:

- [Router](./core-crates/router.md) - Main routing and processing logic
- [API Models](./core-crates/api-models.md) - Data models for API interactions
- [Storage Implementation](./core-crates/storage-impl.md) - Database abstractions

## Utility Crates

These crates provide supporting functionality:

- [Common Utils](./utility-crates/common-utils.md) - Shared utility functions
- [Masking](./utility-crates/masking.md) - Data masking for sensitive information
```

### 3. Alphabetical Index

Used for comprehensive listings where categorical organization is less important than findability.

**Example Structure:**
```markdown
# Connector Index

This document provides an alphabetical listing of all payment connectors supported by Hyperswitch.

## A-E

- [Adyen](./connectors/adyen.md)
- [Authorize.net](./connectors/authorize-net.md)
- [Braintree](./connectors/braintree.md)
- [Checkout.com](./connectors/checkout-com.md)
- [Cybersource](./connectors/cybersource.md)

## F-J

- [Fiserv](./connectors/fiserv.md)
- [Global Payments](./connectors/global-payments.md)
```

## Index File Creation Process

Follow these steps to create an effective index file:

### 1. Plan the Structure

Before creating the index file:

1. **Analyze the Content**
   - Identify all components that will be split into separate files
   - Determine logical groupings or categories
   - Identify the most important or frequently used components

2. **Choose an Index Type**
   - Overview Index: For most topic-based or hierarchical splitting
   - Category Index: For content with clear categorical distinctions
   - Alphabetical Index: For reference material with many similar items

3. **Design the Navigation Flow**
   - Determine how users will navigate between components
   - Plan the navigation hierarchy (breadcrumbs, parent-child relationships)
   - Consider cross-references between related components

### 2. Create the Index File

When creating the index file:

1. **Use Consistent Naming**
   - Name the file `overview.md` for topic-based indices
   - Use descriptive names for category or alphabetical indices (e.g., `crate-index.md`)
   - Place the file at the root of the directory containing the split files

2. **Include Standard Sections**
   - Title: Clear identification of the topic
   - Introduction: Brief overview of the subject matter
   - Purpose: Explanation of what the documentation covers
   - Navigation: Links to all component documents
   - Related Topics: Links to relevant external documentation

3. **Implement Proper Linking**
   - Use relative links to component documents
   - Include descriptive link text
   - Group links logically by category or sequence
   - Consider adding brief descriptions for each link

### 3. Add Metadata and Navigation Aids

To enhance usability:

1. **Include Metadata Header**
   ```markdown
   ---
   title: Payment Flows Overview
   last_updated: 2025-05-27
   position: 1
   related_files:
     - ../../webhooks/payment-webhooks.md
     - ../../configuration/payment-configuration.md
   ---
   ```

2. **Add Table of Contents**
   - Include a table of contents for longer index files
   - Use heading levels appropriately for TOC generation

3. **Create Navigation Aids**
   - Add breadcrumb trails for nested documentation
   - Include "Next" and "Previous" links if there's a logical sequence
   - Add "Up" links to parent indices for nested index structures

## Index File Templates

### Overview Index Template

```markdown
---
title: [Topic] Overview
last_updated: [Date]
position: 1
---

# [Topic] Overview

This document provides an overview of [topic] and serves as an index for detailed documentation.

## Introduction

[Brief introduction to the topic and its importance in the system]

## Components

[Topic] includes the following components:

- [Component 1](./component1.md) - [Brief description]
- [Component 2](./component2.md) - [Brief description]
- [Component 3](./component3.md) - [Brief description]

## Key Concepts

- **[Concept 1]**: [Brief explanation]
- **[Concept 2]**: [Brief explanation]
- **[Concept 3]**: [Brief explanation]

## Common Use Cases

- [Use Case 1](./use-case1.md)
- [Use Case 2](./use-case2.md)

## Related Documentation

- [Related Topic 1](../related-topic1/overview.md)
- [Related Topic 2](../related-topic2/overview.md)
```

### Category Index Template

```markdown
---
title: [Category] Index
last_updated: [Date]
position: 1
---

# [Category] Index

This document categorizes [items] by their function and purpose.

## [Category 1]

[Brief description of this category]

- [Item 1](./category1/item1.md) - [Brief description]
- [Item 2](./category1/item2.md) - [Brief description]
- [Item 3](./category1/item3.md) - [Brief description]

## [Category 2]

[Brief description of this category]

- [Item 4](./category2/item4.md) - [Brief description]
- [Item 5](./category2/item5.md) - [Brief description]
```

## Index Maintenance Process

To keep indices accurate and useful:

### 1. Regular Review

- Schedule quarterly reviews of all index files
- Verify that all links are functional
- Ensure all component documents are properly referenced
- Update metadata (especially last_updated dates)

### 2. Change Management

When adding new component documents:

1. Add appropriate links to the index file
2. Update any categorization or grouping
3. Consider whether the index organization needs adjustment

When removing or relocating component documents:

1. Update or remove links in the index file
2. Add notes about relocated documentation if needed
3. Consider adding redirects for moved content

### 3. Cross-Reference Validation

- Ensure that component documents link back to the index
- Verify that cross-references between components are correct
- Check that breadcrumbs and navigation links are accurate

## Special Cases

### Multi-Level Indices

For complex documentation with multiple levels of splitting:

1. **Create Hierarchy of Indices**
   - Top-level index for the entire topic
   - Second-level indices for major sections
   - Component documents at the lowest level

2. **Implement Clear Navigation**
   - Ensure each index links to both parent and child indices
   - Provide breadcrumb trails showing the full hierarchy
   - Consider adding a visual representation of the hierarchy

### Transitioning From Single File to Split Documents

When splitting an existing document:

1. **Create Transition Index**
   - Initially, include all content in the index file
   - Gradually move sections to component files
   - Update links as content is moved

2. **Add Migration Notes**
   - Include notes about the transition in the index
   - Provide guidance for users familiar with the original structure

## Index Creation Tools

To assist with index creation and maintenance:

### Link Verification

Use link checking tools to verify that all links in index files are valid:

```bash
# Example command for a hypothetical link checker
linkchecker memory-bank/thematic/documentation_process/review_process/README.md
```

### TOC Generation

For large index files, consider using tools to automatically generate tables of contents:

```bash
# Example command for TOC generation
toc-generator memory-bank/thematic/crates/router/overview.md
```

## Case Studies

### Case Study 1: Router Flow Documentation

For the payment flows documentation (`payment_flows.md`, `refund_flows.md`, `webhook_flows.md`):

1. **Create Flows Directory Structure**
   ```
   /flows/
   ├── overview.md                 # Main index for all flows
   ├── payments/                   # Payment flows directory
   │   ├── overview.md             # Payment flows index
   │   └── [component files]
   ├── refunds/                    # Refund flows directory
   │   ├── overview.md             # Refund flows index
   │   └── [component files]
   └── webhooks/                   # Webhook flows directory
       ├── overview.md             # Webhook flows index
       └── [component files]
   ```

2. **Implement Main Flows Index**
   - Provide high-level overview of all flow types
   - Include links to each flow type's index
   - Add diagrams showing relationships between flows

3. **Create Type-Specific Indices**
   - For each flow type, create an index with links to component documents
   - Include sequence diagrams showing the complete flow
   - Provide navigation between flow types

### Case Study 2: Crate Index Reorganization

For the `crateIndex.md` file:

1. **Create Crate Directory Structure**
   ```
   /crates/
   ├── crate-index.md              # Main index with categorization
   ├── core-crates.md              # Details on core crates
   ├── utility-crates.md           # Details on utility crates
   ├── connector-crates.md         # Details on connector crates
   └── supporting-crates.md        # Details on other crates
   ```

2. **Implement Main Crate Index**
   - Categorize crates by function
   - Provide links to category-specific indices
   - Include search guidance for finding specific crates

3. **Create Category-Specific Indices**
   - For each category, create an index with detailed crate information
   - Include dependency relationships between crates
   - Add usage examples for major crates

## Related Documents

- [File Identification Criteria](file_identification_criteria.md)
- [File Splitting Strategy](file_splitting_strategy.md)
- [Cross-Reference Update System](cross_reference_update_system.md) (to be created)
- [File Size Management Guide](../file_size_management_guide.md)
