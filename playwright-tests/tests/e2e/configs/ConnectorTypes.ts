/**
 * Type Definitions for Connector Configurations
 *
 * Shared types across all connector config files
 */

export interface CardDetails {
  card_number: string;
  card_exp_month: string;
  card_exp_year: string;
  card_holder_name: string;
  card_cvc: string;
}

export interface MandateData {
  customer_acceptance: any;
  mandate_type: {
    single_use?: {
      amount: number;
      currency: string;
    };
    multi_use?: {
      amount: number;
      currency: string;
    };
  };
}

export interface PaymentMethodData {
  card?: CardDetails | any;
  bank_redirect?: any;
  bank_transfer?: any;
  wallet?: any;
}

export interface RequestConfig {
  currency?: string;
  amount?: number;
  payment_method?: string;
  payment_method_type?: string;
  payment_method_data?: PaymentMethodData;
  customer_acceptance?: any;
  setup_future_usage?: string;
  authentication_type?: string;
  mandate_data?: MandateData | null;
  billing?: any;
  shipping?: any;
  shipping_cost?: number;
  payment_type?: string;
  amount_to_capture?: number;
  [key: string]: any;
}

export interface ResponseBody {
  status?: string;
  setup_future_usage?: string;
  payment_method?: string;
  payment_method_data?: any;
  attempt_count?: number;
  amount?: number;
  amount_capturable?: number;
  amount_received?: number;
  net_amount?: number;
  shipping_cost?: number;
  error_code?: string;
  error_message?: string;
  unified_code?: string;
  unified_message?: string;
  session_token?: any[];
  deleted?: boolean;
  incremental_authorizations?: any[];
  [key: string]: any;
}

export interface ResponseConfig {
  status: number;
  body?: ResponseBody | any;
}

export interface ExchangeConfig {
  Request?: RequestConfig;
  Response?: ResponseConfig;
  Configs?: {
    [key: string]: {
      specName: string[];
      value: string;
    };
  };
}

export interface ConnectorConfig {
  multi_credential_config?: any;
  card_pm?: {
    [key: string]: ExchangeConfig;
  };
  bank_transfer_pm?: {
    [key: string]: ExchangeConfig;
  };
  bank_redirect_pm?: {
    [key: string]: ExchangeConfig;
  };
  wallet_pm?: {
    [key: string]: ExchangeConfig;
  };
  upi_pm?: {
    [key: string]: ExchangeConfig;
  };
  pm_list?: {
    [key: string]: any;
  };
  [key: string]: any;
}

export interface CustomerAcceptance {
  acceptance_type: string;
  accepted_at: string;
  online: {
    ip_address: string;
    user_agent: string;
  };
}

export interface BillingAddress {
  line1?: string;
  line2?: string;
  line3?: string;
  city?: string;
  state?: string;
  zip?: string;
  country?: string;
  first_name?: string;
  last_name?: string;
}

export interface PhoneDetails {
  number: string;
  country_code: string;
}

export interface RequiredField {
  required_field: string;
  display_name: string;
  field_type: string | { user_full_name?: { first_name: string; last_name: string } };
  value?: string;
}
