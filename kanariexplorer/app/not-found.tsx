import Link from "next/link";
import { ArrowIcon} from "./components/SiteChrome";

export default function NotFound() {
  return (
      <section className="not-found-page section-wrap">
        <div className="not-found-card">
          <p className="section-kicker">404 / PAGE NOT FOUND</p>
          <h1>
            This route
            <br />
            <span>does not exist.</span>
          </h1>
          <p>
            The page may have moved, or the address may be incorrect. Head back
            to the network homepage or continue with the documentation.
          </p>
          <div className="hero-actions">
            <Link className="button button--light" href="/">
              BACK HOME <ArrowIcon />
            </Link>
            <a
              className="button button--ghost"
              href="https://docs.kanarinetwork.site/"
              rel="noreferrer"
              target="_blank"
            >
              READ THE DOCS <ArrowIcon />
            </a>
          </div>
          <i aria-hidden="true" />
        </div>
      </section>
  );
}
