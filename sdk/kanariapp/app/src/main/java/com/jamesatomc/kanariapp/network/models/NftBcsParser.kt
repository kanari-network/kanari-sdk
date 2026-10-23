package com.jamesatomc.kanariapp.network.models

/**
 * Minimal BCS reader for NFT object payloads the RPC did not pre-parse.
 *
 * The client locally decodes supported framework NFT layouts when the RPC
 * leaves `parsed` empty:
 *
 * - **framework `krc_721::Nft`**: `id: UID, collection: address, number: u64,
 *   name: String, description: Option<String>, image_url: Option<Url>,
 *   attributes: VecMap<String, String>`
 * - **legacy framework NFT**: same metadata fields without collection/number
 *
 * BCS has no field names, only ordered fields, so every attempt is just a
 * sequential read of the expected layout; the first one that consumes cleanly
 * (and yields a non-blank name) wins.
 */
object NftBcsParser {

    private class Reader(val bytes: ByteArray) {
        var pos = 0

        fun take(n: Int): ByteArray {
            if (pos + n > bytes.size) throw IllegalArgumentException("EOF at $pos need $n")
            val out = bytes.copyOfRange(pos, pos + n)
            pos += n
            return out
        }

        fun uleb(): Int {
            var value = 0L
            var shift = 0
            while (true) {
                val b = take(1)[0].toInt() and 0xFF
                value = value or ((b and 0x7F).toLong() shl shift)
                if (b and 0x80 == 0) break
                shift += 7
                if (shift > 35) throw IllegalArgumentException("uleb128 too long")
            }
            if (value > Int.MAX_VALUE.toLong()) throw IllegalArgumentException("uleb128 overflow")
            return value.toInt()
        }

        fun skip(n: Int) {
            take(n)
        }

        fun u64String(): String {
            val littleEndian = take(8)
            val bigEndian = littleEndian.reversedArray()
            return java.math.BigInteger(1, bigEndian).toString()
        }
    }

    private fun readString(r: Reader): String {
        val len = r.uleb()
        if (len > 1 shl 22) throw IllegalArgumentException("String too long: $len")
        return String(r.take(len), Charsets.UTF_8)
    }

    private fun readOptional(r: Reader): String? {
        return when (r.take(1)[0].toInt()) {
            0 -> null
            1 -> readString(r)
            else -> throw IllegalArgumentException("bad Option tag")
        }
    }

    private fun readVecMap(r: Reader): List<Pair<String, String>> {
        val len = r.uleb()
        val entries = ArrayList<Pair<String, String>>(len)
        repeat(len) { entries.add(readString(r) to readString(r)) }
        return entries
    }

    private fun finished(r: Reader): Boolean = r.pos == r.bytes.size

    /** Parse `data` into the shared [NftParsedData], or `null` if no layout matches. */
    fun parse(data: ByteArray): NftParsedData? = listOf(
        ::parseKrc721,
        ::parseFrameworkNft,
    ).firstNotNullOfOrNull { attempt ->
        try {
            attempt(data)
        } catch (_: Exception) {
            null
        }
    }

    private fun parseKrc721(data: ByteArray): NftParsedData? {
        val r = Reader(data)
        r.skip(32) // UID
        r.skip(32) // collection address
        val number = r.u64String()
        val name = readString(r)
        val description = readOptional(r)
        val imageUrl = readOptional(r)
        val attributes = readVecMap(r)
        if (!finished(r) || name.isBlank()) return null
        return NftParsedData(
            name = name,
            imageUrl = imageUrl?.takeIf { it.isNotBlank() },
            description = description ?: "",
            number = number,
            attributes = NftAttributes(
                attributes.map { it.first },
                attributes.map { it.second },
            ),
        )
    }

    private fun parseFrameworkNft(data: ByteArray): NftParsedData? {
        val r = Reader(data)
        r.skip(32) // UID
        val name = readString(r)
        val description = readOptional(r)
        val imageUrl = readOptional(r)
        val attributes = readVecMap(r)
        if (!finished(r) || name.isBlank()) return null
        return NftParsedData(
            name = name,
            imageUrl = imageUrl?.takeIf { it.isNotBlank() },
            description = description ?: "",
            number = "",
            attributes = NftAttributes(
                attributes.map { it.first },
                attributes.map { it.second },
            ),
        )
    }
}
