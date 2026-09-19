package com.jamesatomc.kanariapp.wallet.zklogin

import com.sun.net.httpserver.HttpServer
import org.junit.Assert.*
import org.junit.Test
import java.net.InetSocketAddress
import java.util.concurrent.Executors

class ZkLoginHttpTest {

    private fun urlOf(server: HttpServer, path: String) =
        "http://127.0.0.1:${server.address.port}$path"

    @Test
    fun capped_get_accepts_small_body() {
        val server = HttpServer.create(InetSocketAddress("127.0.0.1", 0), 0)
        server.executor = Executors.newCachedThreadPool()
        server.createContext("/jwks") { ex ->
            val body = """{"keys":[]}""".toByteArray()
            ex.sendResponseHeaders(200, body.size.toLong())
            ex.responseBody.use { it.write(body) }
        }
        server.start()
        try {
            assertEquals(
                """{"keys":[]}""",
                ZkLoginAuth.httpGetCapped(urlOf(server, "/jwks"), ZkLoginAuth.MAX_HTTP_BYTES),
            )
        } finally {
            server.stop(0)
        }
    }

    @Test
    fun capped_get_rejects_chunked_flood() {
        val server = HttpServer.create(InetSocketAddress("127.0.0.1", 0), 0)
        server.executor = Executors.newCachedThreadPool()
        server.createContext("/flood") { ex ->
            // 0 = chunked: no Content-Length to trust, streaming cap must fire.
            ex.sendResponseHeaders(200, 0)
            ex.responseBody.use { out ->
                val chunk = ByteArray(8192)
                repeat(64) { out.write(chunk) } // 512 KiB > 256 KiB cap
            }
        }
        server.start()
        try {
            try {
                ZkLoginAuth.httpGetCapped(urlOf(server, "/flood"), ZkLoginAuth.MAX_HTTP_BYTES)
                fail("expected oversize response to fail")
            } catch (e: ZkLoginAuth.AuthException) {
                assertTrue(e.message!!.contains("too large"))
            }
        } finally {
            server.stop(0)
        }
    }

    @Test
    fun capped_get_rejects_lying_content_length() {
        val server = HttpServer.create(InetSocketAddress("127.0.0.1", 0), 0)
        server.executor = Executors.newCachedThreadPool()
        server.createContext("/lie") { ex ->
            // Declared length alone exceeds the cap: reject before reading
            // a single byte (body is well-formed so only the header trips).
            val body = ByteArray(ZkLoginAuth.MAX_HTTP_BYTES + 1)
            ex.sendResponseHeaders(200, body.size.toLong())
            ex.responseBody.use { it.write(body) }
        }
        server.start()
        try {
            try {
                ZkLoginAuth.httpGetCapped(urlOf(server, "/lie"), ZkLoginAuth.MAX_HTTP_BYTES)
                fail("expected lying length to fail")
            } catch (e: ZkLoginAuth.AuthException) {
                assertTrue(e.message!!.contains("too large"))
            }
        } finally {
            server.stop(0)
        }
    }

    @Test
    fun capped_get_rejects_http_errors() {
        val server = HttpServer.create(InetSocketAddress("127.0.0.1", 0), 0)
        server.executor = Executors.newCachedThreadPool()
        server.createContext("/nope") { ex ->
            ex.sendResponseHeaders(500, -1)
            ex.responseBody.use { }
        }
        server.start()
        try {
            try {
                ZkLoginAuth.httpGetCapped(urlOf(server, "/nope"), ZkLoginAuth.MAX_HTTP_BYTES)
                fail("expected HTTP 500 to fail")
            } catch (e: ZkLoginAuth.AuthException) {
                assertTrue(e.message!!.contains("500"))
            }
        } finally {
            server.stop(0)
        }
    }
}
