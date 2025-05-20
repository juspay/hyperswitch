# Hyperswitch Project Brief

## Overview

Hyperswitch is an open-source payments orchestration platform built in Rust. It provides a single API to access the payments ecosystem and its features, enabling businesses to manage their payment stack efficiently.

## Core Components

1.  **Hyperswitch Backend** (the primary Rust application, typically found in the `juspay/hyperswitch` repository): Enables seamless payment processing with comprehensive support for various payment flows - authorization, authentication, void and capture workflows along with robust management of post-payment processes like refunds and chargeback handling.

2.  **SDKs (Frontend)**: Available for multiple platforms, these unify the payment experience across various methods such as cards, wallets, BNPL, bank transfers, and more. Specific SDKs include:
    *   `hyperswitch-web` (for web applications)
    *   `hyperswitch-sdk-android` (for Android applications)
    *   `hyperswitch-sdk-ios` (for iOS applications)

3.  **Control Center**: Enables users to manage the entire payments stack without any coding, typically through a visual interface. It allows the creation of workflows for routing, payment retries, and defining conditions to invoke 3DS, fraud risk management (FRM), and surcharge modules.

## Vision

> Linux for Payments

Hyperswitch serves as a well-architected designed reference platform, built on best-in-class design principles, empowering businesses to own and customize their payment stack. It provides a reusable core payments stack that can be tailored to specific requirements while relying on the Hyperswitch team for enhancements, support, and continuous innovation.

## Core Values

1.  **Embrace Payments Diversity**: Drive innovation in the ecosystem in multiple ways.
2.  **Make it Open Source**: Increase trust; Improve the quality and reusability of software.
3.  **Be community driven**: Enable participatory design and development.
4.  **Build it like Systems Software**: Set a high bar for Reliability, Security and Performance SLAs.
5.  **Maximise Value Creation**: For developers, customers & partners.

## Key Features

### Functional Features

-   **Unified API**: Single API for all payment processors
-   **Multiple Payment Methods**: Cards, Wallets, Bank Transfers, BNPL, etc.
-   **Comprehensive Payment Flows**: Authorization, Authentication, Void, Capture
-   **Post-Payment Processes**: Refunds, Chargebacks
-   **Non-Payment Use Cases**: Integration with external services for enhanced functionality, such as dedicated Fraud Risk Management (FRM) systems or specialized authentication providers.
-   **Optimized Payment Routing**: Success rate-based, rule-based, volume distribution
-   **Fallback Handling**: Automatic fallback mechanisms
-   **Intelligent Retry**: Retry mechanisms for failed payments based on error codes

### Non-Functional Features

-   **High Performance**: Built in Rust for optimal performance
-   **Scalability**: Designed to handle high transaction volumes
-   **Reliability**: Robust error handling and recovery mechanisms
-   **Security**: PCI DSS compliant with secure data handling
-   **Extensibility**: Modular design for easy extension
-   **Observability**: Comprehensive monitoring and logging

## Project Goals

1.  Provide a reusable core payments stack that can be customized to specific requirements
2.  Empower businesses to own and customize their payment stack
3.  Continuously innovate and enhance the platform based on community feedback
4.  Maintain high standards for reliability, security, and performance
5.  Foster a vibrant community of contributors and users

## Links to Related Resources

-   [Hyperswitch Documentation](https://docs.hyperswitch.io/)
-   [GitHub Repository (Backend)](https://github.com/juspay/hyperswitch)
-   [Control Center Repository](https://github.com/juspay/hyperswitch-control-center)
-   [SDK Repository (Web)](https://github.com/juspay/hyperswitch-web)
    *   *(Consider adding direct links to Android and iOS SDK repositories if they are separate and public, e.g., `github.com/juspay/hyperswitch-sdk-android`)*