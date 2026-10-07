import { createContext, useContext, useState, type ReactNode } from "react";
import type { Locale } from "@interview/plugin-sdk";
const en = {
  appName: "Dataroom",
  login: "Sign in",
  logout: "Sign out",
  email: "Email",
  password: "Password",
  demoNotice: "Sign in to your workspace.",
  companyAccount: "Company account",
  investorAccount: "Investor account",
  company: "Company",
  investor: "Investor",
  language: "Language",
  loading: "Loading…",
  connectionError: "Could not connect to the API. Check the server and try again.",
  retry: "Try again",
  loginError: "Could not sign in. Check your credentials, then try again.",
  logoutError: "Could not sign out. Please try again.",
  workspace: "Workspace",
  noWorkspace: "No workspace is available for this account.",
  noPlugin: "This plugin is unavailable.",
  pluginError: "Could not load the plugin. Check the bundle and try again.",
  room: "Data room",
  menu: "Plugins",
  materials: "Materials",
  searchMaterials: "Search materials",
  searchPlaceholder: "Search by title...",
  registerMaterial: "Register material",
  materialTitle: "Title",
  materialTitlePlaceholder: "Enter material title",
  materialFile: "File",
  fileHelp: "Only UTF-8 .txt and .md files (up to 256KB) are supported.",
  registerAction: "Register",
  registering: "Registering...",
  cancel: "Cancel",
  close: "Close",
  noMaterials: "No materials have been registered yet.",
  noSearchResults: "No materials matched your search.",
  statusReady: "Ready",
  statusProcessing: "Processing",
  statusFailed: "Failed",
  fileName: "File name",
  status: "Status",
  createdAt: "Created at",
  viewContent: "View content",
  materialDetail: "Material Details",
  content: "Content",
  registerSuccess: "Material registered successfully.",
  registerError: "Failed to register material. Please try again.",
  fetchError: "Failed to load materials. Please check the connection and try again.",
  fetchDetailError: "Failed to load material detail. Please try again.",
  titleRequired: "Title is required.",
  fileRequired: "Please select a valid .txt or .md file.",
  fileTooLarge: "File size exceeds 256KB limit.",
};
type Strings = Record<keyof typeof en, string>;
const ko: Strings = {
  appName: "Dataroom",
  login: "로그인",
  logout: "로그아웃",
  email: "이메일",
  password: "비밀번호",
  demoNotice: "워크스페이스에 로그인해주세요.",
  companyAccount: "기업 계정",
  investorAccount: "투자자 계정",
  company: "기업 담당자",
  investor: "투자자",
  language: "언어",
  loading: "불러오는 중입니다.",
  connectionError: "API에 연결하지 못했습니다. 서버를 확인하고 다시 시도해주세요.",
  retry: "다시 시도",
  loginError: "로그인하지 못했습니다. 입력한 정보를 확인하고 다시 시도해주세요.",
  logoutError: "로그아웃하지 못했습니다. 다시 시도해주세요.",
  workspace: "워크스페이스",
  noWorkspace: "접근할 수 있는 워크스페이스가 없습니다.",
  noPlugin: "이 플러그인에 접근할 수 없습니다.",
  pluginError: "플러그인을 불러오지 못했습니다. 번들을 확인하고 다시 시도해주세요.",
  room: "데이터룸",
  menu: "플러그인",
  materials: "자료 목록",
  searchMaterials: "자료 검색",
  searchPlaceholder: "제목으로 검색...",
  registerMaterial: "자료 등록",
  materialTitle: "제목",
  materialTitlePlaceholder: "자료 제목을 입력하세요",
  materialFile: "파일",
  fileHelp: "UTF-8 .txt 및 .md 파일(최대 256KB)만 지원합니다.",
  registerAction: "등록하기",
  registering: "등록 중...",
  cancel: "취소",
  close: "닫기",
  noMaterials: "등록된 자료가 없습니다.",
  noSearchResults: "검색 결과가 없습니다.",
  statusReady: "준비 완료",
  statusProcessing: "처리 중",
  statusFailed: "처리 실패",
  fileName: "파일명",
  status: "상태",
  createdAt: "등록일",
  viewContent: "본문 보기",
  materialDetail: "자료 상세",
  content: "본문",
  registerSuccess: "자료가 성공적으로 등록되었습니다.",
  registerError: "자료 등록에 실패했습니다. 다시 시도해주세요.",
  fetchError: "자료를 불러오지 못했습니다. 연결 상태를 확인하고 다시 시도해주세요.",
  fetchDetailError: "자료 상세를 불러오지 못했습니다. 다시 시도해주세요.",
  titleRequired: "제목을 입력해주세요.",
  fileRequired: ".txt 또는 .md 파일을 선택해주세요.",
  fileTooLarge: "파일 크기가 256KB 제한을 초과했습니다.",
};
const Context = createContext<{
  locale: Locale;
  setLocale(value: Locale): void;
  t: Strings;
} | null>(null);
export function LocaleProvider({ children }: { children: ReactNode }) {
  const [locale, setValue] = useState<Locale>("ko");
  return (
    <Context.Provider
      value={{
        locale,
        t: locale === "ko" ? ko : en,
        setLocale(value) {
          document.documentElement.lang = value;
          setValue(value);
        },
      }}
    >
      {children}
    </Context.Provider>
  );
}
export function useI18n() {
  const context = useContext(Context);
  if (!context) throw new Error("LocaleProvider is missing");
  return context;
}
