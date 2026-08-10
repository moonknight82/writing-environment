import { invoke } from "@tauri-apps/api/core";

export interface ReviewReplacement {
  value: string;
}

export interface ReviewContext {
  text: string;
  offset: number;
  length: number;
}

export interface ReviewCategory {
  id: string;
  name: string;
}

export interface ReviewRule {
  id: string;
  description: string;
  issueType: string;
  category: ReviewCategory;
}

export interface ReviewMatch {
  message: string;
  shortMessage: string;
  replacements: ReviewReplacement[];
  offset: number;
  length: number;
  context: ReviewContext;
  rule: ReviewRule;
}

export interface ReviewResult {
  matches: ReviewMatch[];
  checkedCharacters: number;
  requestCount: number;
}

export interface ReviewRequest {
  endpoint: string;
  language: string;
  text: string;
}

export interface ConnectionTestResult {
  address: string;
  languageName: string;
  encrypted: boolean;
  privateNetwork: boolean;
  loopback: boolean;
}

export interface ConnectionTestRequest {
  endpoint: string;
  language: string;
}

export function checkGrammarStyle(request: ReviewRequest): Promise<ReviewResult> {
  return invoke<ReviewResult>("check_grammar_style", { request });
}

export function testLanguageToolConnection(
  request: ConnectionTestRequest,
): Promise<ConnectionTestResult> {
  return invoke<ConnectionTestResult>("test_language_tool_connection", { request });
}
