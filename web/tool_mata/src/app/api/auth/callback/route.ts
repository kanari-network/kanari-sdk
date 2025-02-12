import { NextRequest, NextResponse } from 'next/server';

export async function GET(request: NextRequest) {
  const searchParams = request.nextUrl.searchParams;
  const code = searchParams.get('code');

  if (!code) {
    return NextResponse.redirect('/error?message=No code provided');
  }

  try {
    const tokenResponse = await fetch('https://github.com/login/oauth/access_token', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Accept': 'application/json',
      },
      body: JSON.stringify({
        client_id: process.env.NEXT_PUBLIC_GITHUB_CLIENT_ID,
        client_secret: process.env.GITHUB_CLIENT_SECRET,
        code,
      }),
    });

    if (!tokenResponse.ok) {
      console.error('Token response not ok:', await tokenResponse.text());
      return NextResponse.redirect('/error?message=Authentication failed');
    }

    const tokenData = await tokenResponse.json();
    
    if (tokenData.error) {
      console.error('Token error:', tokenData.error);
      return NextResponse.redirect('/error?message=' + tokenData.error_description);
    }

    // Redirect back to the main page with the token
    return NextResponse.redirect(`${process.env.NEXT_PUBLIC_APP_URL}?token=${tokenData.access_token}`);
  } catch (error) {
    console.error('Authentication error:', error);
    return NextResponse.redirect('/error?message=Authentication failed');
  }
}