# How is `rarreg.key` generated?

WinRAR uses an ECC-based signature algorithm to generate `rarreg.key`. The algorithm it used is a variant of Chinese SM2 digital signature algorithm. Different from many standard ECDSAs, the curve that WinRAR selected is a curve over the composite field $\mathrm{GF}((2^{15})^{17})$.

## 1. Composite field $\mathrm{GF}((2^{15})^{17})$

Elements in the ground field $\mathrm{GF}(2^{15})$ are represented with standard basis, i.e. polynomial basis. The irreducible polynomial is

$$
p(\alpha) = \alpha^{15} + \alpha + 1.
$$

where each coefficient is in $\mathrm{GF}(2)$. If we use

$$
B_1 = \{1, \alpha, \alpha^2, \ldots, \alpha^{14}\}
$$

as the standard basis of the ground field, an element $A \in \mathrm{GF}(2^{15})$ can be denoted as

$$
A = \sum_{i=0}^{14} a_i \alpha^i, \qquad a_i \in \mathrm{GF}(2).
$$

---

The irreducible polynomial of the composite field $\mathrm{GF}((2^{15})^{17})$ is

$$
Q(\beta) = \beta^{17} + \beta^3 + 1
$$

where each coefficient is in $\mathrm{GF}(2^{15})$. If we use

$$
B_2 = (1, \beta, \beta^2, \ldots, \beta^{16})
$$

as the standard basis of the composite field, an element $B \in \mathrm{GF}((2^{15})^{17})$ can be denoted as

$$
B=\sum_{j=0}^{16}\left(\sum_{i=0}^{14} a_{j,i}\alpha^{i}\right)\beta^{j}
=\sum_{j=0}^{16}\sum_{i=0}^{14} a_{j,i}\alpha^{i}\beta^{j},
\qquad a_{j,i}\in GF(2)
$$

---

For clarity, we use $D$, which is a 255-bit-long integer, to denote an element $B \in \mathrm{GF}((2^{15})^{17})$. The map between them is

$$
B=\sum_{j=0}^{16}\left(\sum_{i=0}^{14} a_{j,i}\alpha^{i}\right)\beta^{j}
=\sum_{j=0}^{16}\sum_{i=0}^{14} a_{j,i}\alpha^{i}\beta^{j}
\Longleftrightarrow
D=\sum_{j=0}^{16}\sum_{i=0}^{14} a_{j,i}\cdot 2^{15j+i}
$$

## 2. Elliptic curve over $\mathrm{GF}((2^{15})^{17})$

The equation of the elliptic curve that WinRAR uses is

$$
y^2 + xy = x^3 + x^2 + 161
$$

where \( 161 \in GF((2^{15})^{17}) \).

The base point \( G \) is

$$
G = (G_x, G_y)
$$

其中：

$$
G_x = \texttt{0x56fdcbc6a27acee0cc2996e0096ae74feb1acf220a2341b898b549440297b8cc}
\in GF\left((2^{15})^{17}\right)
$$

$$
G_y = \texttt{0x20da32e8afc90b7cf0e76bde44496b4d0794054e6ea60f388682463132f931a7}
\in GF\left((2^{15})^{17}\right)
$$

A verification example of the base point \( G \) on the curve:

We use order \( k \) as

$$
n = 0x1026dd85081b82314691ced9bbec30547840e4bf72d8b5e0d258442bbcd31,\qquad n \in \mathbb{Z}
$$

## 3. Message hash algorithm

We use

$$
M = m_0 m_1 \cdots m_{l-1}, \qquad m_i \in [0, 256)
$$

to denote a message whose length is $\ell$. So the SHA1 value of $M$ should be

$$
\mathrm{SHA}_1(M) = S_0 \| S_1 \| S_2 \| S_3 \| S_4, \qquad S_i \in [0, 2^{32})
$$

where $s_0, \ldots, s_4$ are the 5 state values when SHA1 outputs. Generally speaking, the final SHA1 value should be the concatenation of these 5 state values while each state value is serialized in big-endian.

However, WinRAR does not serialize the 5 state values. Instead, it uses a big integer $h$ as the hash of the input message:

$$
h = \left( \sum_{i=0}^{4} S_i \cdot 2^{32i} \right) + \texttt{0x1bd10xb4e33c7c0ffd8d43} \cdot 2^{32 \cdot 5}
$$

## 4. ECC digital signature algorithm

We use $k$ to denote private key, and $P$ to denote public key. So there must be

$$
P = k \cdot G
$$

If we use $h$ to denote the hash of input data, WinRAR uses the following algorithm to perform signing:

1. Generate a random big integer $\mathrm{Rnd}$ which satisfies $0 < \mathrm{Rnd} < n$

2. Calculate $r$,

    $$
    r = \left( (Rnd \cdot G)_x + h \right) \bmod n
    $$

   where $(Rnd \cdot G)_x$ means we take the X coordinate of $\mathrm{Rnd}G$ and convert it from $\mathrm{GF}((2^{15})^{17})$ to a big integer.

   If $r = 0$ or $r + \mathrm{Rnd} = n$, go back to step 1.

3. Calculate $s$

    $$
    s = (Rnd - kr) \bmod n
    $$

   If $s = 0$, go back to step 1.

4. Output $(r, s)$.

## 5. WinRAR private key generation algorithm

We use

$$
T = t_0 t_1 \cdots t_{l-1}, \qquad t_i \in [0, 256)
$$

to denote input data whose length is $\ell$. WinRAR uses it to generate private key $k$.

1. We use $g_0, g_1, \ldots, g_5$ to denote six 32-bit integers. So there is

    $$
    g_j = \sum_{i=0}^{3} g_{j,i} \cdot 2^{8i}, \qquad g_{j,i} \in [0, 256)
    $$

2. Let $g_0 = 0$.

3. If $\ell \ne 0$, we calculate the SHA1 value of $T$. Then assign SHA1 state value $S_i$ to $g_{i+1}$:

    $$
    \mathrm{SHA}_1(T) = S_0 \| S_1 \| S_2 \| S_3 \| S_4
    $$
    $$
    g_{i+1} = S_i, \qquad i = 0,1,2,3,4
    $$

   Otherwise, when $\ell = 0$, we let

   $$
   \begin{aligned}
    g_1 &= \texttt{0xeb3eb781}, \\
    g_2 &= \texttt{0x50265329}, \\
    g_3 &= \texttt{0xdc5ef4a3}, \\
    g_4 &= \texttt{0x6847b9d5}, \\
    g_5 &= \texttt{0xcde43b4c}.
    \end{aligned}
   $$

4. Regard $g_0$ as a counter, add itself by 1. Calculate SHA1:

    $$
    \mathrm{SHA}_1(g_{0,0}\|g_{0,1}\|g_{0,2}\|g_{0,3}\|g_{1,0}\|g_{1,1}\|\cdots\|g_{5,0}\|g_{5,1}\|g_{5,2}\|g_{5,3}) = S_0\|S_1\|S_2\|S_3\|S_4
    $$
   We take the lowest 16 bits of $S_0$ and denote it as $k_{g_0}$.

5. Repeat step 4 for 14 more times.

6. After that, we will get $k_1, k_2, \ldots, k_{15}$. Then output private key

    $$
    k = \sum_{i=1}^{15} k_i \cdot 2^{16i}
    $$

## 6. The private key and public key of WinRAR

Private key $k$ is

$$
k = \texttt{0x59fe6abcca90bdb95f0105271fa85fb9f11f467450c1ae9044b7fd61d65e}, \qquad k \in \mathbb{Z}
$$

This private key is generated by the algorithm describled in section 5 where the length of data $T$ is zero.

Public key $P$ is

$$
\begin{aligned}
P &= (P_x, P_y) \\
P_x &= \texttt{0x3861220ed9b36c9753df09a159dfb148135d495db3af8373425ee9a28884ba1a}, \qquad
P_x \in \mathrm{GF}\!\left((2^{15})^{17}\right) \\
P_y &= \texttt{0x12b64e62db43a56114554b0cbd573379338cea9124c8443c4f50e6c8b013ec20}, \qquad
P_y \in \mathrm{GF}\!\left((2^{15})^{17}\right)
\end{aligned}
$$


## 7. Generation of `rarreg.key`

The generation of license file `rarreg.key` requires 2 arguments:

1. Username, an ANSI-encoded string, without null-terminator. Denoted as

    $$
    U = u_0u_1\cdots u_{l-1}
    $$

2. License type, an ANSI-encoded string, without null-terminator. Denoted as

    $$
    L = l_0l_1\cdots l_{l-1}
    $$

The following is the algorithm to generate `rarreg.key`.

1. Use the algorithm described in section 5, with argument $U$, to generate private key $k_U$ and public key $P_U$. Then output hexlified public key string with SM2 compressed public key format. The hexlified public key is denoted as $\mathrm{Temp}$.

   The length of $\mathrm{Temp}$ should be 64. If less, pad with `0` until the length is 64.

2. Let $Data^3$ be

    $$
    Data^3 = "60" \| \| Temp_0 \| \| Temp_1 \| \| \cdots \| \| Temp_{47}
    $$

3. Use the algorithm described in section 5, with argument $\mathrm{Data^3}$, to generate private key $k_{\mathrm{Data^3}}$ and public key $P_{\mathrm{Data^3}}$. Then output hexlified public key string with SM2 compressed public key format. The hexlified public key is denoted as $\mathrm{Data^0}$.

   The length of $\mathrm{Data^0}$ should be 64. If less, pad with `0` until the length is 64.

4. Let $\mathrm{UID}$ be

    $$
    UID = Temp_{48} \| \| Temp_{49} \| \| \cdots \| \| Temp_{63} \| \| Data_{00} \| \| Data_{01} \| \| Data_{02} \| \| Data_{03}
    $$

5. Use the algorithm described in section 4, with argument $L$ and private key $k$ described in section 6, to get signature $(r_L, s_L)$.

   The bit length of $r_L$ and $s_L$ shall not be more than 240. Otherwise, repeat this step.

6. Convert $r_L$ and $s_L$ to hex-integer string $\mathrm{SZ}^{r_L}$ and $\mathrm{SZ}^{s_L}$, without `0x` prefix.

   If the length of $\mathrm{SZ}^{r_L}$ or $\mathrm{SZ}^{s_L}$ is less than 60, pad character `0` until the length is 60.

7. Let $\mathrm{Data^1}$ be

    $$
    Data^1 = "60" \| \| {SZ}^{s_L} \| \| {SZ}^{r_L}
    $$

8. Let $\mathrm{Temp}$ be

    $$
    Temp = U \| \| Data_0
    $$

   Use the algorithm described in section 4, with argument $\mathrm{Temp}$ and private key $k$ described in section 6, to get signature $(r_{\mathrm{Temp}}, s_{\mathrm{Temp}})$.

   The bit length of $r_{\mathrm{Temp}}$ and $s_{\mathrm{Temp}}$ shall not be more than 240. Otherwise, repeat this step.

9. Convert $r_{\mathrm{Temp}}$ and $s_{\mathrm{Temp}}$ to hex-integer string $\mathrm{SZ}r_{\mathrm{Temp}}$ and $\mathrm{SZ}s_{\mathrm{Temp}}$, without `0x` prefix.

   If the length of $\mathrm{SZ}r_{\mathrm{Temp}}$ or $\mathrm{SZ}s_{\mathrm{Temp}}$ is less than 60, pad character `0` until the length is 60.

10. Let $\mathrm{Data^2}$ be

    $$
    Data^2 = "60" \| \| SZ^{sTemp} \| \| SZ^{rTemp}
    $$

11. Calculate CRC32 value of

    $$
    L \| \| U \| \| Data^0 \| \| Data^1 \| \| Data^2 \| \| Data^3
    $$

    The final checksum is the complement of the CRC32 value. Then convert the checksum to decimal string $\mathrm{SZ^{checksum}}$. If the length is less than 10, pad character `0` until the length is 10.

12. Let $\mathrm{Data}$ be

    $$
    Data = Data^0 \| \| Data^1 \| \| Data^2 \| \| Data^3 \| \| SZ^{checksum}
    $$

13. Output with format:

    - A fixed header `RAR registration data`, taking one line.
    - Username, taking one line.
    - License type, taking one line.
    - UID, taking one line, with format:

      $$
      \texttt{"UID="} \| \mathrm{UID}
      $$

    - Output $\mathrm{Data}$, with 54 characters a line.
