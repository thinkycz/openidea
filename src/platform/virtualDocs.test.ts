import { describe, expect, it, vi } from "vitest";
import { virtualDocument, isVirtualUri, virtualContainer, virtualName, virtualParent } from "./virtualDocs";
import { basename, dirname, relativeDir, relativePath } from "./project";

const requestVirtual = vi.hoisted(() => vi.fn());
vi.mock("./ipc", async (original) => {
  const actual = await original<typeof import("./ipc")>();
  return { ...actual, commands: { ...actual.commands, lspVirtualDocument: requestVirtual } };
});

const JDT =
  "jdt://contents/spring-boot-4.1.1.jar/org.springframework.boot/SpringApplication.class?=demo/%5C/m2%5C/spring-boot-4.1.1.jar%3Corg.springframework.boot%28SpringApplication.class";

describe("virtual document URIs", () => {
  it("tells URIs from file paths", () => {
    expect(isVirtualUri(JDT)).toBe(true);
    expect(isVirtualUri("untitled:Untitled-1")).toBe(true);
    expect(isVirtualUri("/Users/me/a.java")).toBe(false);
    expect(isVirtualUri("C:\\src\\a.java")).toBe(false);
    expect(isVirtualUri("file:///a.java")).toBe(false);
    expect(isVirtualUri(null)).toBe(false);
  });

  it("names and locates library classes", () => {
    expect(virtualName(JDT)).toBe("SpringApplication.class");
    expect(virtualParent(JDT)).toBe("jdt://contents/spring-boot-4.1.1.jar/org.springframework.boot");
    expect(virtualContainer(JDT)).toBe("spring-boot-4.1.1.jar › org.springframework.boot");
  });

  it("makes path helpers URI-aware", () => {
    expect(basename(JDT)).toBe("SpringApplication.class");
    expect(basename(dirname(JDT))).toBe("org.springframework.boot");
    expect(relativePath("/p", JDT)).toBe("spring-boot-4.1.1.jar › org.springframework.boot › SpringApplication.class");
    expect(relativeDir("/p", JDT)).toBe("spring-boot-4.1.1.jar › org.springframework.boot");
    expect(relativePath("/p", "/p/src/A.java")).toBe("src/A.java");
  });
});

it("re-fetches PHP builtins so a changed target is not hidden by the frontend cache", async () => {
  const uri = "phpantom-stub://DateTime?project_context=57";
  const doc = { uri, title: "DateTime.php", text: "<?php class DateTime {}", languageId: "php", decompiled: false };
  requestVirtual.mockResolvedValueOnce({ status: "ok", data: { ...doc, description: "PHP 8.2" } });
  requestVirtual.mockResolvedValueOnce({ status: "ok", data: { ...doc, description: "PHP 8.3" } });
  expect((await virtualDocument(uri)).description).toBe("PHP 8.2");
  expect((await virtualDocument(uri)).description).toBe("PHP 8.3");
});

it("keeps caching immutable library documents", async () => {
  requestVirtual.mockClear();
  const doc = {
    uri: "jdt://contents/immutable-test.jar/C.class",
    title: "C.java",
    text: "class C {}",
    languageId: "java",
    decompiled: true,
  };
  requestVirtual.mockResolvedValue({ status: "ok", data: doc });
  expect(await virtualDocument(doc.uri)).toEqual(doc);
  expect(await virtualDocument(doc.uri)).toEqual(doc);
  expect(requestVirtual).toHaveBeenCalledOnce();
});
